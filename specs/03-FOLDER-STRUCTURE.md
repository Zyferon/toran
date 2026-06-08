# FOLDER & FILE STRUCTURE
## The Complete Repository Layout

### Root Level
```
toran/
├── README.md                 # Project overview, install instructions, quick start
├── LICENSE                   # MIT license (permissive, commercial-friendly)
├── CHANGELOG.md              # Version history, breaking changes, migration notes
├── CONTRIBUTING.md           # How to contribute, code standards, PR process
├── CODE_OF_CONDUCT.md        # Community standards
├── SECURITY.md               # Security policy, how to report vulnerabilities, PGP key
├── Makefile                  # Common commands: build, test, lint, release
├── docker-compose.yml        # Optional: spins up Redis + Postgres + Toran core + dashboard
├── .gitignore                # Ignore build artifacts, secrets, local databases
├── .github/                  # GitHub-specific files
│   ├── workflows/            # CI/CD pipelines
│   │   ├── rust-ci.yml       # Rust test, lint, audit on every PR
│   │   ├── python-ci.yml     # Python test, lint, type-check on every PR
│   │   ├── release.yml       # Build wheels and binaries on version tag
│   │   └── docs.yml          # Deploy documentation site on main branch push
│   ├── ISSUE_TEMPLATE/       # Bug report, feature request templates
│   └── PULL_REQUEST_TEMPLATE.md
├── docs/                     # Documentation source (Mintlify or Nextra)
│   ├── introduction.md
│   ├── quickstart.md
│   ├── policies.md           # How to write policy files
│   ├── architecture.md       # Deep dive into internals
│   ├── deployment.md         # Self-hosting guide
│   ├── sdk-reference.md      # Python API reference
│   └── faq.md
├── policies/                 # Example policy files for common use cases
│   ├── email-guardian.yml    # Email-related protections
│   ├── database-guardian.yml  # Database write protections
│   ├── financial-guardian.yml # Financial transaction protections
│   └── minimal.yml           # Single-rule example for tutorials
├── scripts/                  # Development and release scripts
│   ├── setup-dev.sh          # Install Rust, Python, dependencies
│   ├── build-wheels.sh       # Cross-compile Python wheels
│   ├── benchmark.sh          # Run latency benchmarks
│   └── migrate-db.sh         # Database migration helper
└── src/                      # Source code (monorepo)
```

### Rust Core (`src/core/`)
```
src/core/
├── Cargo.toml                # Rust package manifest, dependencies, features
├── Cargo.lock                # Pinned dependency versions (committed for reproducibility)
├── build.rs                  # Build script (compile FlatBuffers schemas, link static libs)
├── src/
│   ├── main.rs               # Entry point: CLI argument parsing, runtime initialization
│   ├── lib.rs                # Library entry point (when core is used as a library, not binary)
│   ├── config.rs             # Configuration loading: TOML files, environment variables, defaults
│   ├── server.rs             # TCP/Unix socket listener, connection handler spawn
│   ├── runtime.rs            # Tokio runtime initialization, thread pool configuration
│   ├── policy/
│   │   ├── mod.rs            # Policy module public interface
│   │   ├── loader.rs         # File system watcher, hot-reload logic
│   │   ├── parser.rs         # YAML to AST conversion (uses serde_yaml)
│   │   ├── compiler.rs       # AST to decision tree optimization (flatten nested rules)
│   │   ├── evaluator.rs      # Decision tree traversal, rule matching, condition evaluation
│   │   ├── schema.rs         # Policy data structures (Rule, Condition, Action, etc.)
│   │   └── validator.rs      # JSON Schema validation against policy definitions
│   ├── engine/
│   │   ├── mod.rs            # Engine module public interface
│   │   ├── request.rs        # Incoming request data structures (function name, args, context)
│   │   ├── response.rs       # Outgoing response (ALLOW, BLOCK, REQUIRE_APPROVAL)
│   │   ├── evaluator.rs      # Top-level evaluation orchestrator (loads policy, calls evaluator)
│   │   └── context.rs        # Request context builder (agent ID, session, timestamp, risk signals)
│   ├── state/
│   │   ├── mod.rs            # State manager abstraction
│   │   ├── manager.rs        # StateManager trait (interface)
│   │   ├── sqlite.rs         # SQLite implementation (default, embedded)
│   │   ├── redis.rs          # Redis implementation (production, multi-instance)
│   │   ├── postgres.rs       # PostgreSQL implementation (enterprise, analytics)
│   │   └── memory.rs         # In-memory implementation (testing, ephemeral)
│   ├── notification/
│   │   ├── mod.rs            # Notification module public interface
│   │   ├── dispatcher.rs     # Event router (sends events to all configured adapters)
│   │   ├── adapter.rs        # Adapter trait definition (interface)
│   │   ├── slack.rs          # Slack webhook adapter
│   │   ├── email.rs          # SMTP and API email adapter
│   │   ├── webhook.rs        # Generic HTTP POST adapter
│   │   ├── discord.rs        # Discord webhook adapter
│   │   └── teams.rs          # Microsoft Teams webhook adapter
│   ├── protocol/
│   │   ├── mod.rs            # FlatBuffers protocol module
│   │   ├── request.fbs       # FlatBuffers schema for request messages
│   │   ├── response.fbs      # FlatBuffers schema for response messages
│   │   ├── event.fbs         # FlatBuffers schema for notification events
│   │   └── generated/        # Auto-generated Rust code from FlatBuffers schemas
│   ├── api/
│   │   ├── mod.rs            # REST API module (Axum)
│   │   ├── router.rs         # Route definitions (GET /health, POST /approvals, etc.)
│   │   ├── handlers.rs       # HTTP handler implementations
│   │   ├── middleware.rs     # Auth, logging, rate limiting, CORS
│   │   ├── models.rs         # Request/response JSON structs (serde)
│   │   └── websocket.rs      # Real-time WebSocket for live approval queue updates
│   ├── security/
│   │   ├── mod.rs            # Security utilities
│   │   ├── crypto.rs         # Cryptographic helpers (hashing, HMAC, random tokens)
│   │   ├── sandbox.rs        # Policy expression sandbox (prevents code injection in conditions)
│   │   └── audit.rs          # Audit log writer (tamper-evident logging)
│   ├── metrics/
│   │   ├── mod.rs            # Metrics and telemetry
│   │   ├── prometheus.rs     # Prometheus metrics exporter
│   │   └── statsd.rs         # StatsD metrics exporter (optional)
│   └── cli/
│       ├── mod.rs            # Command-line interface
│       ├── args.rs           # CLI argument definitions (clap crate)
│       └── commands.rs       # CLI command implementations (start, validate, status, migrate)
├── tests/
│   ├── integration_tests.rs  # End-to-end tests (spawn core, send requests, verify responses)
│   ├── policy_tests.rs       # Policy evaluation edge cases
│   ├── state_tests.rs        # State manager tests (all three backends)
│   └── benchmark_tests.rs    # Latency and throughput benchmarks
└── benches/
    ├── policy_eval.rs        # Criterion benchmark for policy evaluation speed
    └── throughput.rs         # Custom benchmark for requests per second
```

### Python SDK (`src/sdk/`)
```
src/sdk/
├── pyproject.toml            # Python package manifest (PEP 621), dependencies, build system
├── setup.py                  # Legacy setup script (for compatibility)
├── MANIFEST.in               # Include non-Python files in the wheel (YAML schemas, etc.)
├── Cargo.toml                # PyO3 extension manifest (Maturin build)
├── src/
│   ├── toran/
│   │   ├── __init__.py       # Public API exports (gate, configure, exceptions)
│   │   ├── core.py           # Pure Python: decorator implementation, async integration
│   │   ├── config.py         # Configuration loading (YAML, env vars, defaults)
│   │   ├── client.py         # Communication client (talks to Rust core via socket)
│   │   ├── exceptions.py     # Custom exceptions (BlockedError, DeniedError, TimeoutError)
│   │   ├── state.py          # Python-side state tracking (maps function calls to approval IDs)
│   │   ├── types.py          # Type definitions (TypedDict, Protocol for type checking)
│   │   └── utils.py          # Utility functions (serialization, hashing, validation)
│   ├── toran/_internal/      # Private implementation (not imported by users)
│   │   ├── __init__.py
│   │   ├── _rust_bridge.py   # PyO3 extension wrapper (loads compiled Rust module)
│   │   ├── _flatbuffers.py   # FlatBuffers serialization helpers
│   │   └── _async.py         # Async event loop integration (asyncio compatibility)
│   └── rust/                 # Rust source for the PyO3 extension
│       ├── lib.rs            # PyO3 module definition (#[pymodule])
│       ├── bridge.rs         # Python-callable Rust functions
│       ├── serializer.rs     # FlatBuffers serialization in Rust (faster than Python)
│       └── socket.rs         # Unix/TCP socket client in Rust (faster than Python sockets)
├── tests/
│   ├── test_decorator.py     # Unit tests for the @gate decorator
│   ├── test_client.py        # Tests for Rust core communication
│   ├── test_exceptions.py    # Tests for error handling and recovery
│   ├── test_integration.py   # End-to-end tests with a real Rust core process
│   └── test_async.py         # Tests for async/await compatibility
├── examples/
│   ├── minimal.py            # Single decorator on a simple function
│   ├── langchain_example.py  # Integration with LangChain agents
│   ├── crewai_example.py     # Integration with CrewAI agents
│   ├── pydantic_ai_example.py # Integration with Pydantic AI
│   ├── autogen_example.py    # Integration with AutoGen
│   └── custom_framework.py   # Integration with a custom Python loop
└── benchmarks/
    ├── latency.py            # Measure decorator overhead in microseconds
    └── throughput.py         # Measure concurrent function calls per second
```

### Web Dashboard (`src/dashboard/`)
```
src/dashboard/
├── package.json              # Node.js dependencies, scripts
├── next.config.js            # Next.js configuration (output: standalone for Docker)
├── tsconfig.json             # TypeScript configuration
├── tailwind.config.ts        # Tailwind CSS configuration (design tokens)
├── postcss.config.js         # PostCSS configuration
├── .env.example              # Example environment variables
├── src/
│   ├── app/                  # Next.js App Router (React Server Components)
│   │   ├── layout.tsx        # Root layout (global providers, fonts, metadata)
│   │   ├── page.tsx          # Landing page (marketing, install instructions)
│   │   ├── globals.css       # Global styles, Tailwind directives
│   │   ├── dashboard/
│   │   │   ├── layout.tsx    # Dashboard layout (sidebar, header)
│   │   │   ├── page.tsx      # Dashboard home (approval queue summary)
│   │   │   ├── queue/
│   │   │   │   ├── page.tsx  # Live approval queue (table + real-time updates)
│   │   │   │   └── [id]/
│   │   │   │       └── page.tsx # Individual approval detail view
│   │   │   ├── policies/
│   │   │   │   ├── page.tsx  # Policy file browser (list all policies)
│   │   │   │   └── [name]/
│   │   │   │       └── page.tsx # Policy editor (YAML with syntax highlighting)
│   │   │   ├── audit/
│   │   │   │   └── page.tsx  # Audit log browser (searchable, filterable table)
│   │   │   ├── settings/
│   │   │   │   └── page.tsx  # Team settings, notification adapters, integrations
│   │   │   └── api-keys/
│   │   │       └── page.tsx  # API key management
│   │   └── api/              # API routes (Next.js route handlers)
│   │       ├── health/
│   │       │   └── route.ts  # Health check endpoint
│   │       ├── approvals/
│   │       │   ├── route.ts  # List approvals (GET), create approval action (POST)
│   │       │   └── [id]/
│   │       │       └── route.ts # Get approval detail, approve/deny
│   │       ├── policies/
│   │       │   └── route.ts  # List policies, validate policy content
│   │       └── webhooks/
│   │           └── route.ts  # Receive webhooks from notification adapters
│   ├── components/           # Reusable React components
│   │   ├── ui/               # Base UI components (shadcn/ui)
│   │   │   ├── button.tsx
│   │   │   ├── card.tsx
│   │   │   ├── table.tsx
│   │   │   ├── dialog.tsx
│   │   │   ├── input.tsx
│   │   │   ├── badge.tsx
│   │   │   └── toast.tsx
│   │   ├── approval-queue.tsx      # Main approval queue table component
│   │   ├── approval-card.tsx       # Individual approval item card
│   │   ├── policy-editor.tsx       # Monaco Editor wrapper for YAML editing
│   │   ├── decision-tree.tsx       # React Flow visualization of agent decisions
│   │   ├── audit-log-table.tsx     # Audit log with filtering and pagination
│   │   ├── risk-score-badge.tsx    # Color-coded risk score indicator
│   │   ├── slack-connect-button.tsx # OAuth connection for Slack
│   │   ├── notification-config.tsx # Adapter configuration form
│   │   └── team-member-list.tsx    # Team management table
│   ├── hooks/                # Custom React hooks
│   │   ├── use-approvals.ts  # Fetch and subscribe to approval data
│   │   ├── use-policies.ts   # Fetch and mutate policy files
│   │   ├── use-audit-logs.ts # Fetch audit logs with filters
│   │   ├── use-websocket.ts  # WebSocket connection for real-time updates
│   │   └── use-auth.ts       # Authentication state and session
│   ├── lib/                  # Utility libraries
│   │   ├── api.ts            # Typed API client (fetch wrapper with error handling)
│   │   ├── auth.ts           # Clerk authentication helpers
│   │   ├── utils.ts          # General utilities (date formatting, class merging)
│   │   └── constants.ts      # App constants (API base URL, page sizes, timeouts)
│   ├── types/                # TypeScript type definitions
│   │   ├── approval.ts       # Approval request/response types
│   │   ├── policy.ts         # Policy file types
│   │   ├── audit.ts          # Audit log entry types
│   │   ├── notification.ts   # Notification adapter types
│   │   └── api.ts            # Generic API response types
│   └── styles/
│       └── globals.css       # Tailwind imports, custom CSS variables
├── public/                   # Static assets
│   ├── logo.svg              # Toran logo (monochrome, scalable)
│   ├── favicon.ico
│   └── og-image.png          # Open Graph image for social sharing
└── tests/
    ├── e2e/                  # Playwright end-to-end tests
    │   ├── queue.spec.ts     # Test approval queue functionality
    │   └── policies.spec.ts  # Test policy editor functionality
    └── unit/                   # Jest unit tests for components
        └── components/
            └── approval-card.test.tsx
```

### Documentation Site (`docs/` or separate `src/docs/`)
```
src/docs/
├── mint.json                 # Mintlify configuration (navigation, theme, analytics)
├── introduction.md           # What is Toran, who it is for, 2-minute video embed
├── quickstart.md             # 5-minute setup guide (pip install, write policy, decorate function)
├── concepts/
│   ├── policies.md           # Deep dive into policy files, rules, conditions, actions
│   ├── evaluation.md         # How the Rust core evaluates policies (decision tree)
│   ├── blocking.md           # How the async blocking mechanism works
│   ├── notifications.md      # How human approval notifications work
│   └── audit-trails.md       # How audit logs are generated and stored
├── guides/
│   ├── langchain.md          # Integration guide for LangChain
│   ├── crewai.md             # Integration guide for CrewAI
│   ├── pydantic-ai.md        # Integration guide for Pydantic AI
│   ├── autogen.md            # Integration guide for AutoGen
│   ├── custom-framework.md   # Integration guide for custom Python scripts
│   ├── slack-setup.md        # How to connect Slack notifications
│   ├── email-setup.md        # How to configure email notifications
│   └── self-hosting.md       # How to deploy Toran on your own server
├── reference/
│   ├── sdk-api.md            # Python SDK API reference (auto-generated from docstrings)
│   ├── cli-commands.md       # Rust core CLI reference
│   ├── policy-schema.md      # JSON Schema for policy files
│   └── configuration.md      # Environment variables and config file options
├── deployment/
│   ├── docker.md             # Docker deployment guide
│   ├── kubernetes.md         # Kubernetes deployment guide (Helm chart)
│   ├── systemd.md            # Linux systemd service setup
│   └── cloud-providers.md    # AWS, GCP, Azure deployment notes
├── contributing/
│   ├── setup.md              # Development environment setup
│   ├── architecture.md       # Internal architecture for contributors
│   ├── rust-guidelines.md    # Rust coding standards
│   ├── python-guidelines.md  # Python coding standards
│   └── testing.md            # How to run tests, write tests, benchmark
└── changelog.md              # Auto-generated from core CHANGELOG.md
```

### Infrastructure & Deployment (`infra/`)
```
infra/
├── docker/
│   ├── core.Dockerfile       # Rust core container (distroless, 15MB)
│   ├── dashboard.Dockerfile  # Next.js dashboard container (standalone output)
│   └── docker-compose.yml    # Full stack: core + dashboard + Redis + Postgres
├── kubernetes/
│   ├── helm-chart/           # Helm chart for Kubernetes deployment
│   │   ├── Chart.yaml
│   │   ├── values.yaml
│   │   └── templates/
│   │       ├── deployment.yaml
│   │       ├── service.yaml
│   │       ├── configmap.yaml
│   │       └── ingress.yaml
│   └── raw-manifests/        # Plain Kubernetes YAML (for non-Helm users)
│       ├── namespace.yaml
│       ├── deployment.yaml
│       ├── service.yaml
│       └── ingress.yaml
├── terraform/                # Infrastructure as Code (optional, for managed hosting)
│   ├── main.tf
│   ├── variables.tf
│   └── outputs.tf
└── scripts/
    ├── install.sh            # One-line installer: curl | bash (detects OS, downloads binary)
    ├── upgrade.sh            # In-place upgrade script
    └── backup.sh             # Database backup script
```

### Benchmarks & Performance (`benches/`)
```
benches/
├── rust/
│   ├── policy_eval.rs        # Criterion.rs benchmark: policy evaluation latency
│   ├── throughput.rs         # Custom benchmark: requests per second under load
│   └── memory.rs             # Heap profiling: memory usage per 1000 concurrent requests
├── python/
│   ├── decorator_overhead.py # Measure microseconds added by @gate decorator
│   └── concurrency.py        # Measure how many suspended functions Python can hold
└── dashboard/
    └── load_test.js          # Artillery.js load test for dashboard API endpoints
```
