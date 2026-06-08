# PHASE 2: THE PYTHON SDK
## Week 3: The Bridge Between Python and Rust

### Goal
A Python package that users install with `pip install toran` and use with one decorator:
```python
from toran import gate

@gate()
def send_email(to, subject, body):
    # This body only runs after approval
    return mailgun.send(to, subject, body)
```

### What You Build

#### Component 1: The Decorator
The `@gate()` decorator is a Python function that wraps another function. When the wrapped function is called, the decorator intercepts the call before the function body executes. It collects:
- The function name (as a string)
- All positional arguments (as a list)
- All keyword arguments (as a dictionary)
- The current timestamp
- A session ID (from thread-local storage or async context)
- An agent ID (from configuration or environment variable)

The decorator then calls the Rust bridge (Component 2) and waits for a decision. If the decision is ALLOW, it calls the original function body and returns the result. If BLOCK, it raises `BlockedError`. If REQUIRE_APPROVAL, it enters an async wait state.

The decorator supports both sync and async functions. For sync functions, it uses `asyncio.run()` internally if an event loop is available, or falls back to a blocking call with a timeout. For async functions, it returns an awaitable coroutine.

#### Component 2: The Rust Bridge (PyO3 Extension)
A compiled Rust library that Python imports as a module. It exposes three functions to Python:
1. `evaluate(function_name, args, kwargs, context) -> Decision`: Sends the request to the Rust core via socket, returns the decision
2. `wait_for_approval(approval_id, timeout) -> bool`: Blocks until the approval is resolved, returns True for approved, False for denied
3. `connect(socket_path) -> None`: Establishes the connection to the Rust core

The Rust bridge uses PyO3's `#[pyfunction]` macro to expose Rust functions to Python. It uses Maturin to build the Python wheel. The wheel includes the compiled Rust shared library (.so on Linux, .dylib on macOS, .dll on Windows) alongside the Python code.

The bridge uses FlatBuffers to serialize data. The Python side uses the `flatbuffers` Python package to build the request buffer. The Rust side reads the buffer directly. This is faster than JSON and more compact than pickle.

#### Component 3: The Connection Client
A Python class that manages the Unix socket connection to the Rust core. It handles:
- Connection pooling (reuses the same socket for multiple requests)
- Reconnection (if the Rust core restarts, the client reconnects automatically)
- Health checking (sends a ping every 30 seconds to keep the connection alive)
- Timeout handling (if the Rust core does not respond in 5 seconds, raises ConnectionError)

The client uses Python's `asyncio` for non-blocking I/O. It does not block the Python interpreter while waiting for a response.

#### Component 4: The Async Integration
When the decorator receives REQUIRE_APPROVAL, it needs to suspend the function without blocking the thread. For async functions, this is natural: the decorator returns an `await` expression that sleeps until a signal arrives.

For sync functions, the decorator uses `asyncio.run()` to run the wait in a temporary event loop, or falls back to a threading-based wait with a condition variable. The sync path is less efficient but necessary for users who have not adopted async Python.

The async integration stores the pending approval in a thread-safe dictionary keyed by approval ID. When the Rust bridge receives a resolution signal, it looks up the approval ID, sets the result, and wakes the waiting coroutine or thread.

#### Component 5: The Configuration Loader
A Python module that loads Toran configuration from:
- Environment variables (TORAN_SOCKET_PATH, TORAN_POLICY_DIR, etc.)
- A YAML configuration file (`toran.yaml` or `.toran/config.yaml`)
- Python code (`toran.configure(socket_path="...", policy_dir="...")`)

The configuration loader uses the `pydantic` library for validation. It ensures all required fields are present and types are correct. It provides sensible defaults (socket path defaults to `/tmp/toran.sock`, policy directory defaults to `./policies`).

#### Component 6: Exception Handling
Custom exception classes that users can catch:
- `BlockedError`: The function was blocked by policy. The agent can catch this and try an alternative.
- `DeniedError`: The function was denied by a human reviewer. The agent should handle this as a permanent failure.
- `TimeoutError`: No human responded within the timeout window. The agent should retry or escalate.
- `ConnectionError`: The Rust core is not running or not reachable. The agent should fall back to safe mode.

All exceptions include the approval ID, the function name, the policy rule that triggered, and a timestamp. This information is useful for logging and debugging.

### What You Do NOT Build in Phase 2
- No framework-specific integrations (LangChain, CrewAI) yet
- No built-in notification adapters (Slack, email)
- No dashboard interaction from the SDK
- No metrics or telemetry
- No caching layer

### Success Criteria
- `pip install toran` works on Linux, macOS, and Windows (x86_64 and ARM64)
- The decorator adds under 2 milliseconds of overhead for ALLOW decisions
- The decorator can suspend 1,000 concurrent functions without crashing Python
- The SDK reconnects automatically if the Rust core restarts
- All exceptions are catchable and informative

### Human Tasks (Pratik + Dipendra)
- Pratik: Write the Rust bridge (PyO3 extension). This is the hardest part. Read the PyO3 guide thoroughly.
- Dipendra: Write the Python decorator, configuration loader, and exception classes. Write integration tests.
- Both: Test the SDK against a running Rust core. Verify end-to-end flow.

### AI Assistance
- Use AI to generate the PyO3 boilerplate (module definition, function signatures)
- Use AI to generate the FlatBuffers Python serialization code
- Use AI to suggest edge cases for the async integration (what happens if the coroutine is cancelled?)
- Do NOT use AI to design the exception hierarchy. That affects user experience. Design it yourself.
