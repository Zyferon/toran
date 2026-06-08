# PHASE 1: THE RUST CORE ENGINE
## Week 1-2: Build the Gatekeeper

### Goal
A single Rust binary that can:
1. Load a YAML policy file and compile it into a fast decision tree
2. Listen on a Unix socket for incoming requests from Python
3. Evaluate a request against the policy in under 1 millisecond
4. Return ALLOW, BLOCK, or REQUIRE_APPROVAL
5. If REQUIRE_APPROVAL, store the suspended request in SQLite and wait for a signal

### What You Build

#### Component 1: The Policy Loader
A file system watcher that monitors a directory of YAML policy files. When a file changes, it parses the YAML into an abstract syntax tree (AST), validates the AST against a JSON Schema, compiles the AST into a flattened decision tree, and atomically swaps the old tree for the new one in memory. Active requests use the old tree until the swap completes. New requests use the new tree immediately. No downtime. No restarts.

The watcher uses the `notify` crate, which hooks into Linux inotify, macOS FSEvents, or Windows ReadDirectoryChangesW. It debounces rapid changes (if you save a file 3 times in 2 seconds, it only reloads once).

#### Component 2: The Policy Compiler
The compiler takes the AST and produces a read-only decision tree. The tree is a directed acyclic graph where each node is a rule condition (function name match, argument match, context match). The compiler performs these optimizations:
- Rule ordering: Most specific rules first (exact function name match before wildcard patterns)
- Condition flattening: Nested AND/OR conditions are flattened into a single evaluation pass
- Hash-based lookup: Rules are indexed by function name hash, so the evaluator does not scan every rule for every request
- Pre-computed regex: Pattern matching rules compile their regex at load time, not evaluation time

The compiled tree lives in an `Arc<RwLock<DecisionTree>>`. The evaluator holds a read lock for microseconds. The loader holds a write lock for milliseconds during swap. Because the tree is immutable once compiled, read locks are contention-free.

#### Component 3: The Evaluator
The evaluator receives a request struct (function name, arguments as key-value pairs, context like agent ID and timestamp) and traverses the decision tree. It performs:
1. Hash lookup: Find the rule bucket for this function name
2. Condition evaluation: Check each condition in the rule (string equality, numeric comparison, regex match, set membership)
3. Action resolution: If all conditions match, return the rule's action (ALLOW, BLOCK, REQUIRE_APPROVAL)
4. Default fallback: If no rule matches, return the default action (configurable, usually BLOCK for safety)

The evaluator is a pure function. It has no side effects. It allocates no memory. It only reads from the pre-compiled tree and the incoming request. This is why it is sub-millisecond.

#### Component 4: The Socket Server
A Tokio async task that binds to a Unix domain socket (or TCP socket on Windows) and listens for connections from the Python SDK. Each connection spawns a new Tokio task. The task reads FlatBuffers-encoded requests from the socket, calls the evaluator, and writes FlatBuffers-encoded responses back.

The server uses Tokio's `tokio::net::UnixListener` and `tokio::io::AsyncReadExt` / `AsyncWriteExt`. It handles backpressure by limiting the number of concurrent connections (configurable, default 10,000). If the limit is reached, new connections wait in the kernel backlog.

#### Component 5: The State Manager (SQLite)
When the evaluator returns REQUIRE_APPROVAL, the socket server task writes the request details to a SQLite database. The database schema is:
- `approvals` table: id, function_name, arguments_json, agent_id, session_id, status (pending/approved/denied/timeout), created_at, resolved_at, resolved_by
- `audit_log` table: id, event_type, function_name, arguments_json, agent_id, policy_rule_matched, decision, timestamp

The SQLite database is a single file on disk. It is opened with WAL mode (Write-Ahead Logging), which allows readers and writers to proceed concurrently without locking. The Rust core uses the `rusqlite` crate with the `bundled` feature, which compiles SQLite directly into the binary. No separate SQLite installation required.

#### Component 6: The Signal Handler
A separate Tokio task listens for HTTP webhooks (from the dashboard or Slack buttons) or WebSocket messages. When a signal arrives ("approve request ID 12345"), the task updates the SQLite record and sends a message through a Tokio channel to the waiting socket server task. The socket server task then writes the resolution back to the Python SDK connection.

If the Python SDK connection has already closed (the agent process crashed), the signal handler marks the request as orphaned and logs it.

### What You Do NOT Build in Phase 1
- No Redis support (SQLite only)
- No PostgreSQL support
- No Slack/email notifications (console logging only)
- No dashboard (CLI only)
- No metrics or Prometheus
- No authentication
- No risk scoring

### Success Criteria
- The Rust binary compiles to a single static executable under 10MB
- Policy evaluation latency is under 1 millisecond at p99 (measured with Criterion.rs)
- The binary can handle 10,000 concurrent suspended requests without memory growth over 100MB
- Hot-reloading a policy file does not drop active requests
- The binary starts in under 500 milliseconds from cold boot

### Human Tasks (Pratik)
- Write the Rust code for all six components
- Write integration tests that spawn the binary, send requests, and verify responses
- Write Criterion benchmarks for evaluation latency
- Write the CLI interface (start, validate, status commands)
- Review every line of AI-generated code. If you cannot explain it to Dipendra, rewrite it.

### AI Assistance
- Use Cursor or Claude to generate boilerplate (Tokio setup, SQLite schema, FlatBuffers schema definitions)
- Use AI to suggest test cases (edge cases in policy evaluation, race conditions in hot-reload)
- Do NOT use AI to design the decision tree data structure. That is the core moat. Design it yourself.
