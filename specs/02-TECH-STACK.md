# TECHNOLOGY STACK
## Why Each Piece Exists

### The Rust Core (The Engine)
**Language**: Rust, edition 2024, compiled with LLVM optimizations.

**Why Rust**: We need memory safety without garbage collection. Go has a garbage collector that adds unpredictable pause times. Python is 100x slower for tight loops. C++ is unsafe by default. Rust gives us:
- Zero-cost abstractions (policy evaluation compiles to machine code as fast as hand-written C)
- Memory safety at compile time (no segfaults, no use-after-free, no data races)
- Async runtime (Tokio) that handles 100,000 concurrent suspended functions without breaking a sweat
- Single binary deployment (compile once, copy the binary, run anywhere)

**Why Not Go**: Go's garbage collector introduces stop-the-world pauses. When you have 10,000 agent functions suspended waiting for approval, a 10ms GC pause is unacceptable. Go is also slower at CPU-bound tasks like parsing YAML and evaluating boolean expressions. Rust has no GC. Predictable latency is the feature.

**Why Not C++**: C++ would be faster in raw benchmarks, but we are two undergrads building this in 4 weeks. One memory bug in the blocking layer means a production agent hangs forever. Rust's borrow checker prevents this class of bug at compile time. The 5% performance penalty vs C++ is worth the 500% reliability gain.

**Why Not Zig or Nim**: Not enough library ecosystem. We need async runtimes, HTTP clients, YAML parsers, and WebSocket libraries that are production-tested. Rust has these. Zig and Nim do not yet.

### The Async Runtime: Tokio
Tokio is the industry-standard async runtime for Rust. It provides:
- Multi-threaded scheduler that distributes work across all CPU cores
- Zero-allocation channels for communication between components
- Timeouts and intervals for managing approval windows
- TCP and Unix socket listeners for the Python SDK to connect

We use Tokio's multi-threaded runtime with work-stealing. If one core is busy evaluating a complex policy, another core picks up the next request. No request waits in a global queue.

### Serialization: FlatBuffers (Google)
When the Python SDK sends data to the Rust core, we use FlatBuffers, not JSON. FlatBuffers is a zero-copy serialization format from Google. The Rust core can read the data directly from the wire buffer without parsing, without allocating, without copying. This saves 50-100 microseconds per request compared to JSON. At 10,000 requests per second, that is the difference between 1% CPU usage and 30% CPU usage.

We do not use Protocol Buffers because FlatBuffers is faster for read-heavy workloads. We do not use MessagePack because it still requires parsing. FlatBuffers is the only format where the deserialized data structure is literally a pointer cast into the wire buffer.

### Policy Format: YAML with JSON Schema Validation
Humans write policies in YAML because it is the most readable configuration format. But YAML is slow to parse. So we do not parse it on every request.

The Rust core loads all policy files at startup, parses them into an in-memory AST (abstract syntax tree), and compiles them into a decision tree. The compiled tree lives in a read-locked Arc (atomic reference-counted pointer). When a request arrives, the Rust core only traverses the pre-compiled tree. No parsing. No allocation. Just pointer chasing through a read-only structure.

YAML parsing happens once, at startup or when a file changes. We use a file watcher (notify crate) to detect policy changes and hot-reload the compiled tree without restarting the process.

### State Management: SQLite (Default) / Redis (Production) / Postgres (Enterprise)
For the local-first version, we use SQLite. SQLite is a single-file database that requires no server, no port, no configuration. It is the most deployed database in the world. It lives inside the Rust process via the rusqlite crate. Suspended functions, approval history, and audit logs all write to a local .db file.

For production multi-instance deployments, users can configure Redis. Redis is an in-memory data structure store with sub-millisecond latency. It is perfect for tracking which function is waiting for which approval, because it is fast enough that the overhead of network round-trip is still under 2ms.

For enterprise customers who need durable audit trails and SQL analytics, we support PostgreSQL. But Postgres is not the default because it requires a server, credentials, and network configuration. Toran defaults to zero-configuration.

### The Python SDK: Pure Python + PyO3 Bridge
The Python SDK has two parts:
1. A pure Python package (`toran`) that users install via pip. It contains the decorator, the configuration loader, and the async integration.
2. A compiled Rust extension (built with PyO3 and Maturin) that handles the low-latency communication with the Rust core.

The pure Python part is what users import. The compiled extension is what makes it fast. Users do not need to know Rust. They `pip install toran` and the wheel contains the pre-compiled Rust binary for their platform (Linux, macOS, Windows, x86_64, ARM64).

We use PyO3 (not ctypes or CFFI) because it provides safe, zero-overhead bindings between Python and Rust. The Rust extension exposes a Python class that the decorator calls. The class methods are Rust functions that serialize data with FlatBuffers and send it over a local socket.

### The Web Dashboard: Next.js 15 + React Server Components
The dashboard is a separate web application. It is not required for Toran to function. It is a convenience layer for teams who want a visual approval queue, policy editor, and audit log browser.

We use Next.js 15 with React Server Components because:
- Server Components render on the server, reducing client-side JavaScript by 70% compared to traditional React SPAs
- The App Router gives us nested layouts and parallel routes for the approval queue and policy editor
- TypeScript integration is first-class, preventing an entire class of runtime bugs
- Vercel deployment is one-click, but the dashboard can also self-host on any Node.js server

We do not use Vue or Svelte because the React ecosystem has the best data visualization libraries (React Flow for decision trees, TanStack Table for audit logs). We do not use Angular because it is too heavy for a dashboard that should feel lightweight.

### The API Layer: Rust (Axum) or Go (Gin) — Decision Pending
The backend API that serves the dashboard can be built in Rust (Axum) or Go (Gin). Both are valid. Here is the tradeoff:
- **Rust (Axum)**: Same language as the core. Shared data structures. No serialization overhead between core and API. But Rust web development is slower (compile times, borrow checker fights, smaller ecosystem of middleware).
- **Go (Gin)**: Faster to develop. Huge ecosystem of middleware (auth, logging, rate limiting). But requires a separate process and gRPC/JSON bridge to the Rust core. Adds 2-5ms of latency for dashboard queries.

**Recommendation**: Start with Rust (Axum) because the team is already learning Rust for the core. Adding a second language (Go) splits focus. If the API development becomes a bottleneck, rewrite the API in Go later. The API is stateless; it can be replaced without touching the core.

### Authentication: Clerk (Hosted) or Custom JWT (Self-Hosted)
For the hosted dashboard, we use Clerk (clerk.com) because it handles OAuth, SAML, MFA, and user management out of the box. It saves 4 weeks of development time.

For self-hosted deployments, users can configure their own JWT provider or skip auth entirely (single-team mode). Toran does not force authentication complexity on solo developers.

### Notifications: Webhooks + Pluggable Adapters
The notification layer is a plugin system. The Rust core emits an event ("approval required for function X"), and adapters consume that event.

Built-in adapters:
- Slack (via incoming webhooks)
- Email (via SMTP or SendGrid/Resend API)
- Generic HTTP webhook (POST to your own URL)
- Discord
- Microsoft Teams

Users configure adapters in a YAML file. They can write custom adapters in Python if the built-ins do not cover their needs. The adapter interface is simple: receive an event struct, do something, return success or failure.

### Deployment: Docker (Optional) + Single Binary (Default)
The Rust core compiles to a single static binary. No libc dependencies. No runtime. Copy it to a server and run it. This is the default deployment mode.

For users who prefer containers, we provide a Docker image based on distroless (Google's minimal container image). The image is 15MB. It contains only the Rust binary and CA certificates. No shell. No package manager. Minimal attack surface.

### CI/CD: GitHub Actions
- Rust CI: cargo test, cargo clippy (linting), cargo fmt (formatting), cargo audit (security vulnerabilities)
- Python CI: pytest, mypy (type checking), ruff (linting), black (formatting)
- Cross-compilation: The Rust extension builds wheels for Linux, macOS, and Windows automatically on every release tag
- Release: Maturin builds Python wheels, Cargo builds the Rust binary, both attach to the GitHub release
