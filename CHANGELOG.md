# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/) and
this project adheres to [Semantic Versioning](https://semver.org/).

Repository: [github.com/Zyferon/toran](https://github.com/Zyferon/toran)

## [0.1.0] - 2026-06-05

### Added
- Phase 1: Rust core engine. YAML policy loader, hot-reload, compiled
  decision-tree evaluator (sub-millisecond), Unix socket server with
  JSON-line protocol, SQLite state manager (WAL mode, append-only
  audit log), Tokio-based signal handler.
- Phase 2: Python SDK with `@gate` decorator (sync + async),
  thread-safe socket client, configuration loader (env + TOML),
  exception hierarchy, framework integrations for LangChain /
  CrewAI / Pydantic AI / AutoGen.
- Phase 3: Axum REST API. `GET /api/health`, `GET /api/approvals`,
  `GET /api/approvals/:id`, `POST /api/approvals/:id/{approve,deny}`,
  `GET /api/policies`, `GET /api/policies/:name`, `GET /api/audit`,
  `GET /api/summary`, `GET /api/metrics` (Prometheus), `POST
  /webhooks/toran` (HMAC-validated).
- Phase 4: Embedded HTML/JS dashboard. Live approval queue, audit
  log browser, policy browser, resolve buttons. Vanilla JS, no build
  step, assets inlined into the binary.
- Phase 5: Notification dispatcher with three adapters — console
  (always on), Slack (Block Kit), generic webhook (HMAC-signed).
- CLI: `start`, `validate`, `status`, `list`, `approve`, `deny`.
- Example policies: `email-guardian`, `database-guardian`,
  `financial-guardian`, `minimal`, `allow-all`.
- Integration tests: 10 policy, 7 state, 3 end-to-end (socket +
  HTTP), 6 Python (decorator behaviour). All passing.
- Criterion benchmark for policy evaluation (`cargo bench`).
- README, CONTRIBUTING, LICENSE, this CHANGELOG.

### Security
- HMAC-SHA256 webhook signatures.
- 128-bit CSPRNG notify tokens.
- Constant-time token comparison.
- Optional tamper-evident audit-log hash chain.
- Unix socket permission `0660` on creation.
- No code evaluation in policies; fixed operator whitelist.

### Known limitations
- Polling wait for approval (200 ms interval). Replace with a
  broadcast channel for higher-throughput multi-instance deployments.
- `regex` crate does not support look-around. Use explicit allow /
  deny rules instead of negative lookahead.
- Single-process SQLite. For multi-instance use, swap in Postgres
  (the trait is already there).
- Notification retries: 1 attempt only. Add exponential backoff in
  Phase 6.
