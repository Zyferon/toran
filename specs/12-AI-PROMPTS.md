# AI CODING PROMPTS
## How to Use AI Without Losing Control

### Philosophy
AI writes the boring 70%. You write the critical 30%. You review 100%. If you cannot explain a function to an investor, rewrite it.

### Prompt 1: Generate Rust Core Boilerplate
```
I am building a Rust async TCP server using Tokio. The server listens on a Unix socket, accepts connections, reads FlatBuffers-encoded messages, and dispatches them to a handler. Generate the boilerplate for:
1. The main.rs entry point with Tokio runtime initialization
2. A Unix socket listener with connection limit (10,000 concurrent)
3. A connection handler spawn pattern
4. A simple request/response echo handler as a placeholder
5. Graceful shutdown on SIGTERM

Requirements:
- Use tokio::net::UnixListener
- Use tokio::sync::Semaphore for connection limiting
- Use tracing for structured logging
- Use anyhow for error handling
- The code must compile with Rust 2024 edition
- Do NOT include business logic. Only infrastructure.
```

### Prompt 2: Generate PyO3 Bridge
```
I need a PyO3 Rust extension that exposes three functions to Python:
1. `evaluate(function_name: str, args: dict, kwargs: dict, context: dict) -> dict` — sends data to a Rust core via Unix socket, returns a decision dict
2. `wait_for_approval(approval_id: str, timeout: float) -> bool` — blocks until signal
3. `connect(socket_path: str) -> None` — establishes connection

Generate the PyO3 boilerplate for:
- The lib.rs module definition with #[pymodule]
- The three #[pyfunction] definitions with type signatures
- A Rust struct that manages the Unix socket connection (connect, send, receive)
- Error handling that converts Rust errors to Python exceptions

Requirements:
- Use pyo3 0.22
- Use anyhow for Rust error handling
- Use thiserror for custom error types
- The socket communication uses raw bytes (not JSON)
- Do NOT implement the actual socket I/O. Only the struct definition and method stubs.
```

### Prompt 3: Generate Policy Evaluator
```
I need a Rust policy evaluator that matches incoming requests against a set of rules. Each rule has:
- function_name_pattern: exact string or regex
- conditions: list of AND conditions (key, operator, value)
- action: ALLOW, BLOCK, or REQUIRE_APPROVAL

Generate the Rust data structures and a simple evaluator function:
1. Define the Rule, Condition, and Action enums/structs
2. Define a compiled Policy struct that holds a Vec<Rule>
3. Write an evaluate(request: &Request, policy: &Policy) -> Action function
4. The evaluator checks rules in order and returns the first match
5. If no rule matches, return BLOCK as default

Requirements:
- Use regex crate for pattern matching
- Use serde for deserialization (the rules come from YAML)
- The evaluator must be a pure function (no side effects, no allocation)
- Include unit tests for exact match, regex match, and no match cases
- Do NOT include YAML parsing. Assume the Policy is already constructed.
```

### Prompt 4: Generate Next.js Dashboard Page
```
I need a Next.js 15 page (App Router) that displays a real-time approval queue. The page should:
1. Fetch initial data from an API endpoint (GET /api/approvals)
2. Connect to a WebSocket for real-time updates
3. Display a table with columns: function name, arguments summary, time waiting, action buttons
4. Use React Server Components for the initial data fetch
5. Use a client component for the WebSocket connection and interactive buttons
6. Use shadcn/ui Table, Button, and Badge components
7. Use TanStack Table for sorting and filtering

Generate:
- The page.tsx file
- The client component for the table
- The WebSocket hook
- The API route handler (route.ts)

Requirements:
- Use TypeScript with strict mode
- Use Tailwind CSS for styling
- Handle WebSocket reconnection on disconnect
- Handle loading and error states
- Do NOT include actual API calls. Use mock data for the table.
```

### Prompt 5: Generate Security Test Cases
```
I need security test cases for an AI agent approval system. The system has:
- A Rust core that evaluates policies and blocks/allows function calls
- A Python SDK that wraps functions with a @gate decorator
- A WebSocket API that receives approval/denial signals
- A webhook endpoint that receives Slack approval callbacks

Generate test cases for:
1. Bypass attempts (calling the original function without the decorator)
2. Policy injection (modifying policy files to allow blocked actions)
3. Approval spoofing (sending fake approval signals without valid tokens)
4. Denial of service (flooding with requests, suspending too many functions)
5. Eavesdropping (reading Unix socket traffic)
6. SQL injection in policy conditions (if conditions support string matching)

For each test case, provide:
- The attack scenario
- The expected system behavior (what should happen)
- A Rust or Python test function that simulates the attack
- The assertion that validates the defense

Requirements:
- Use Rust's built-in test framework for core tests
- Use Python's pytest for SDK tests
- Include both positive and negative test cases
- Do NOT generate actual attack code that could be used maliciously. Focus on the defense validation.
```

### Prompt 6: Generate Documentation
```
I am writing documentation for an open-source developer tool called Toran. It is a runtime gatekeeper for AI agents. I need the following sections:

1. A one-paragraph introduction that explains what Toran does and why it matters
2. A 5-step quickstart guide (install, write policy, decorate function, run agent, approve)
3. A comparison table: Toran vs Langfuse vs Braintrust vs LangGraph HITL
4. A FAQ with 10 questions (installation, performance, security, pricing, framework support)

Requirements:
- Tone: technical but approachable. No marketing fluff. No buzzwords.
- Audience: Python developers building AI agents.
- Format: Markdown.
- The comparison table must be honest. Do not claim superiority where there is none.
- The FAQ must answer real questions that developers would ask, not hypothetical ones.
```

### Prompt 7: Generate Benchmark Harness
```
I need a Rust benchmark using Criterion.rs that measures the latency of a policy evaluation function. The function takes a request struct and returns an Action enum. The policy has 100 rules with mixed exact matches and regex patterns.

Generate:
1. The benchmark function that calls evaluate() with a prepared request and policy
2. The setup code that constructs the policy and request before the benchmark loop
3. A throughput benchmark that measures how many evaluations per second the function can handle
4. A memory benchmark that measures heap allocations per evaluation (using dhat or a custom allocator)

Requirements:
- Use criterion 0.5
- Use black_box to prevent compiler optimization from removing the evaluation
- The benchmark must run for at least 10 seconds to get stable results
- Include instructions for running the benchmark and interpreting the output
- Do NOT include the actual evaluate function. Only the benchmark harness.
```

### Prompt 8: Generate CI/CD Pipeline
```
I need GitHub Actions workflows for a monorepo with:
- A Rust core (in src/core/)
- A Python SDK (in src/sdk/)
- A Next.js dashboard (in src/dashboard/)

Generate three workflow files:
1. rust-ci.yml: Run on PRs that touch src/core/. Steps: checkout, install Rust, cargo test, cargo clippy, cargo fmt --check, cargo audit.
2. python-ci.yml: Run on PRs that touch src/sdk/. Steps: checkout, install Python, pip install, pytest, mypy, ruff check, black --check.
3. release.yml: Run on version tags (v*). Steps: build Rust binary for Linux/macOS/Windows, build Python wheels for multiple platforms, build Docker images, create GitHub release with assets.

Requirements:
- Use GitHub Actions best practices (caching, matrix builds, artifact upload)
- The Rust workflow should cache cargo dependencies
- The Python workflow should cache pip dependencies
- The release workflow should use cross-compilation for ARM64 targets
- Do NOT include deployment to production. Only build and release artifacts.
```

### How to Use These Prompts
1. Copy the prompt into Cursor, Claude, or ChatGPT.
2. Review the generated code line by line.
3. Ask AI to explain any line you do not understand.
4. Modify the code to fit your architecture.
5. Write the tests yourself. AI is bad at writing tests that actually catch bugs.
6. Commit the code with a message that explains what it does and why.
