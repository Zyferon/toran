<div align="center">

# 🛕 Toran

### The runtime human-approval gatekeeper for AI agents

**Framework-agnostic · Sub-millisecond · Self-hosted · One decorator**

[![Rust CI](https://github.com/Zyferon/toran/actions/workflows/ci.yml/badge.svg)](https://github.com/Zyferon/toran/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange.svg?logo=rust)](https://www.rust-lang.org)
[![Python](https://img.shields.io/badge/Python-3.10%2B-3776AB.svg?logo=python&logoColor=white)](https://www.python.org)
[![Edition](https://img.shields.io/badge/edition-2024-93450a.svg?logo=rust)](https://doc.rust-lang.org/edition-guide/)
[![Binary](https://img.shields.io/badge/binary-5.8%20MB-success.svg)](#-architecture)
[![Status](https://img.shields.io/badge/status-v0.1.0-brightgreen.svg)](./CHANGELOG.md)

[Quick start](#-quick-start) ·
[How it works](#-how-it-works) ·
[Policies](#-policies) ·
[SDK](#-the-python-sdk) ·
[Dashboard](#-the-web-dashboard) ·
[Security](#-security-model)

</div>

---

> **Toran is a gatekeeper.** It sits between your AI agent and the real world. When the agent tries to send an email, write to a database, or call an API, Toran checks a [policy](./policies/). If the action is **allowed**, it executes immediately. If it is **risky**, Toran pauses and asks a human for approval — via the dashboard, Slack, email, or any custom webhook.

Toran works with **any** Python agent framework — 🦜 LangChain, 🚣 CrewAI, 🧩 Pydantic AI, 🤖 AutoGen, or a plain `for` loop. One decorator. No rewrite.

```python
from toran import gate

@gate()
def send_email(to, subject, body):
    return mailgun.send(to, subject, body)

send_email("alice@company.com", "Lunch?", "1pm?")          # ✅ ALLOW   — runs instantly
send_email("stranger@evil.xyz", "Hi", "...")               # ⏸️  REQUIRE_APPROVAL — pings a human
send_email("ceo@x.com", "FREE MONEY winner!!!", "...")     # ⛔ BLOCK   — never sent
```

---

## 📑 Table of contents

- [✨ Why Toran](#-why-toran)
- [🚀 Quick start](#-quick-start)
- [⚙️ How it works](#-how-it-works)
- [📜 Policies](#-policies)
- [🐍 The Python SDK](#-the-python-sdk)
- [🖥️ The web dashboard](#-the-web-dashboard)
- [🔔 Notifications (Slack, webhooks)](#-notifications-slack-webhooks)
- [🏗️ Architecture](#-architecture)
- [📂 Project layout](#-project-layout)
- [🔧 Configuration](#-configuration)
- [🚢 Deployment](#-deployment)
- [🔐 Security model](#-security-model)
- [⌨️ CLI reference](#-cli-reference)
- [🤝 Contributing](#-contributing)
- [📄 License](#-license)

> 🧭 **First time?** Read [`TOUR.md`](./TOUR.md) for a 5-minute guided walkthrough.

---

## ✨ Why Toran

| | |
| --- | --- |
| 🪶 **Framework-agnostic** | One `@gate()` decorator. Works with LangChain, CrewAI, Pydantic AI, AutoGen, or no framework at all. |
| ⚡ **Sub-millisecond** | Compiled decision-tree evaluator. Low single-digit **microseconds** for 1,000 rules on modern x86_64. |
| 📦 **Single static binary** | 5.8 MB. No external services. Runs on a Raspberry Pi. |
| 🔒 **Self-hosted** | Your hardware, your data. Nothing leaves your machine unless you wire up a webhook. |
| 🔥 **Hot-reloaded policies** | Edit a YAML file; the change is live. No restart. |
| 🧾 **Tamper-evident audit log** | Append-only, optionally hash-chained with `SHA-256(prev || row)`. |

---

## 🚀 Quick start

### 1️⃣ Build the core

```bash
git clone https://github.com/Zyferon/toran.git
cd toran
cargo build --release
```

The single static binary is at `target/release/toran`.

### 2️⃣ Write a policy

```yaml
# policies/email-guardian.yaml
name: email-guardian
default_action: ALLOW
rules:
  - name: block-spam
    tool: { exact: send_email }
    conditions:
      - key: subject
        op: regex
        value: '(?i)(viagra|free money|lottery winner)'
    action: BLOCK
  - name: approve-external
    tool: { exact: send_email }
    conditions:
      - key: to
        op: regex
        value: '^[^@]+@[^@]+\.(io|xyz|top|click)$'
    action: REQUIRE_APPROVAL
    timeout_secs: 300
  - name: allow-internal
    tool: { exact: send_email }
    action: ALLOW
```

### 3️⃣ Start the core

```bash
./target/release/toran start
# → 2026-…  toran socket server listening socket=/tmp/toran.sock
# → 2026-…  toran api listening  addr=127.0.0.1:7878
```

Open **http://127.0.0.1:7878** for the dashboard.

### 4️⃣ Decorate your agent

```bash
pip install toran-sdk
```

```python
from toran import gate, configure

configure(socket_path="/tmp/toran.sock", agent_id="prod-agent-1")

@gate()
def send_email(to, subject, body):
    return mailgun.send(to, subject, body)

# That's it. The decorator handles the rest.
send_email("alice@company.com", "Lunch?", "1pm?")           # → ALLOW
send_email("stranger@evil.xyz", "Hi", "...")                # → approval needed
send_email("ceo@x.com", "FREE MONEY winner!!!", "...")      # → BLOCK
```

---

## ⚙️ How it works

Toran is a five-layer system. Every layer is open source and runs on your hardware.

| Layer | Component | Technology |
| --- | --- | --- |
| 1️⃣ Policy | YAML rules, hot-reloaded | `serde_yaml`, `notify` |
| 2️⃣ Evaluation | Compiled decision tree | `regex`, `HashMap` |
| 3️⃣ Blocking | Tokio async wait | `tokio`, `mpsc` |
| 4️⃣ Notification | Slack / webhook / console | `reqwest` shim, `tracing` |
| 5️⃣ Resolution | Dashboard / webhooks | `axum` |

A request flows like this:

```
agent code → @gate → Unix socket → Rust core → policy eval
                                              ├─ ✅ ALLOW → original function runs
                                              ├─ ⛔ BLOCK → BlockedError
                                              └─ ⏸️ REQUIRE_APPROVAL → Slack/email
                                                  human clicks Approve
                                                  → core wakes the future
                                                  → original function runs
```

---

## 📜 Policies

Policies live in `./policies/` (override with `TORAN_POLICY_DIR`). Each `*.yaml` file is one policy. A policy has:

```yaml
name: my-policy                       # required, unique
description: ...                      # optional
priority: 0                           # higher wins when policies overlap
default_action: ALLOW                 # what to do if nothing matches
rules:
  - name: ...                         # required
    tool:                             # required
      exact: send_email               #   string OR
      glob: "send_*"                  #   glob OR
      regex: "^db_.*$"                #   regex
    conditions:                       # optional; all must match
      - key: amount
        op: gt                        # eq, ne, contains, starts_with, ends_with,
                                       # regex, gt, lt, gte, lte, in, not_in, exists
        value: 1000
    action: REQUIRE_APPROVAL          # ALLOW | BLOCK | REQUIRE_APPROVAL
    timeout_secs: 300                 # only for REQUIRE_APPROVAL
    risk_score: 80                    # 0-100
```

The bundled example policies cover common use cases:

- 📧 `email-guardian.yaml` — wire transfer, spam filter, external TLD.
- 🗄️ `database-guardian.yaml` — DROP/DELETE/TRUNCATE protection.
- 💰 `financial-guardian.yaml` — large-amount transfer approval.
- 🔹 `minimal.yaml` — single rule for the quickstart.
- 🟢 `allow-all.yaml` — permissive fallback (`priority: -10`).

`validate` checks syntax and structure:

```bash
./toran validate
# ✓ default action: Allow
# ✓ policy `email-guardian` (5 rules)
# ✓ policy `database-guardian` (4 rules)
# ✓ policy `financial-guardian` (3 rules)
# ✓ policy `allow-everything` (1 rules)
# ✓ policy `minimal` (1 rules)
```

---

## 🐍 The Python SDK

Install from PyPI:

```bash
pip install toran-sdk
```

```python
from toran import gate, configure, BlockedError, DeniedError, TimeoutError

configure(
    socket_path="/var/run/toran.sock",
    agent_id="prod-agent-7",
    fail_open=False,            # raise on connection failure (safer)
)

@gate(policy="email-guardian", timeout_secs=120)
def send_email(to, subject, body):
    return mailgun.send(to, subject, body)

@gate()
async def run_agent():
    # The decorator works on async functions too. Awaiting the call
    # triggers the gate; the event loop is never blocked.
    return await some_async_tool()
```

### 🔌 Framework integrations

```python
from toran.integrations import (
    ToranTool, wrap_crewai_tool, wrap_pydantic_ai_tool, wrap_autogen_function,
)

# 🦜 LangChain
from langchain.tools import MoveFileTool
safe_move = ToranTool(MoveFileTool(), policy="filesystem-guardian")

# 🚣 CrewAI
safe_tool = wrap_crewai_tool(crewai_tool_instance)

# 🧩 Pydantic AI
@pydantic_ai.tool
@wrap_pydantic_ai_tool
def my_tool(ctx, x: int) -> int: ...

# 🤖 AutoGen
agent.register_function("send_email", wrap_autogen_function(send_email))
```

### ⚠️ Exceptions

| Exception | When |
| --- | --- |
| `BlockedError` | The policy forbids this call. Catch and try an alternative. |
| `DeniedError` | A human reviewer denied. Treat as permanent failure. |
| `TimeoutError` | No one answered in time. Retry or escalate. |
| `ToranConnectionError` | The Rust core is not running. Fall back to safe mode. |
| `ConfigurationError` | Mis-configuration. Check `toran.configure(...)`. |

---

## 🖥️ The web dashboard

Open **http://127.0.0.1:7878** after starting the core. The dashboard is a single-page app served by the core (no Node.js, no build step). It provides:

- ✅ **Approval queue** — live list of pending requests, click to approve or deny.
- 📋 **Audit log** — every decision, filter by function or agent.
- 📜 **Policy browser** — read-only YAML viewer with syntax highlighting.
- 📊 **Health / metrics** — uptime, total decisions, average eval latency, Prometheus-format `/api/metrics`.

The HTML/JS/CSS are embedded into the binary at compile time via `include_str!`. No static files, no asset pipeline.

---

## 🔔 Notifications (Slack, webhooks)

```bash
# Slack incoming webhook
export TORAN_SLACK_WEBHOOK="https://hooks.slack.com/services/..."

# Generic HMAC-signed webhook
export TORAN_GENERIC_WEBHOOK="https://my-app.example.com/toran"
```

When a function hits a `REQUIRE_APPROVAL` rule, the dispatcher:

1. Logs the request to stdout (console adapter, always on).
2. Posts a Block-Kit message to Slack (if configured).
3. POSTs the full event JSON to your webhook (if configured).
4. Writes a row to the audit log.

The webhook payload includes `toran_signature` (HMAC-SHA256 of the body using `TORAN_HMAC_SECRET`). Verify it on the receiver.

To resolve from the webhook receiver, POST:

```bash
curl -X POST http://127.0.0.1:7878/webhooks/toran \
  -H 'content-type: application/json' \
  -d '{"approval_id":"...","decision":"approve","token":"<notify_token>","resolved_by":"alice"}'
```

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  USER'S PYTHON AGENT CODE                                   │
│  from toran import gate                                     │
│  @gate()                                                    │
│  def my_action(...): ...                                    │
└──────────────────┬──────────────────────────────────────────┘
                   │ JSON line over Unix socket
                   ▼
┌─────────────────────────────────────────────────────────────┐
│  PYTHON SDK (pure Python)                                   │
│  - Decorator intercepts call                                │
│  - Snapshots args (JSON), context (agent_id, session_id)    │
│  - Calls client.evaluate()                                  │
│  - On REQUIRE_APPROVAL: client.wait_for_approval()          │
└──────────────────┬──────────────────────────────────────────┘
                   │ AF_UNIX, SOCK_STREAM
                   ▼
┌─────────────────────────────────────────────────────────────┐
│  RUST CORE (single binary)                                  │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐         │
│  │ Policy       │ │ Evaluator    │ │ SQLite       │         │
│  │ Loader       │ │ (compiled)   │ │ State        │         │
│  │ + file watch │ │ sub-ms       │ │ WAL mode     │         │
│  └──────────────┘ └──────────────┘ └──────────────┘         │
│  ┌──────────────┐ ┌──────────────┐                          │
│  │ Notification │ │ Metrics      │                          │
│  │ Dispatcher   │ │ (Prometheus) │                          │
│  └──────────────┘ └──────────────┘                          │
└──────────────────┬──────────────────────────────────────────┘
                   │ HTTP (axum)
                   ▼
┌─────────────────────────────────────────────────────────────┐
│  DASHBOARD  +  WEBHOOK  RECEIVERS  +  EXTERNAL SYSTEMS      │
└─────────────────────────────────────────────────────────────┘
```

The Rust core is a single static binary of **5.8 MB**. It uses no external services and can run on a Raspberry Pi. The evaluator does ~100 ns of work per rule; theoretical max throughput is on the order of millions of evaluations per second, bounded in practice by per-connection Tokio task parallelism.

---

## 📂 Project layout

```
toran/
├── src/                    # Rust source (~2,600 lines, 17 files)
│   ├── main.rs             # entry point
│   ├── lib.rs              # library exports
│   ├── config.rs           # config loader (env + TOML)
│   ├── cli.rs              # clap subcommands
│   ├── policy/             # YAML schema, loader, compiler, evaluator, validator
│   ├── state/              # SQLite + memory state managers
│   ├── server.rs           # Unix socket server (Python SDK)
│   ├── protocol.rs         # wire format
│   ├── api/                # axum REST API + embedded dashboard
│   ├── notification/       # dispatcher + Slack/webhook/console
│   ├── security.rs         # HMAC, tokens, chain hashing
│   └── metrics.rs          # Prometheus exporter
├── assets/                 # dashboard CSS/JS (embedded)
├── policies/               # example YAML policies
├── sdk/                    # Python SDK
│   ├── toran/              # package
│   │   ├── __init__.py
│   │   ├── core.py         # @gate decorator
│   │   ├── client.py       # socket client
│   │   ├── config.py
│   │   ├── exceptions.py
│   │   └── integrations.py
│   ├── tests/              # pytest
│   └── examples/           # minimal, langchain, custom
├── benches/                # Criterion benchmarks
├── tests/                  # integration + e2e tests
└── specs/                  # design specs (15 markdown files)
```

---

## 🔧 Configuration

Toran reads from environment variables (and optionally a TOML file passed via `--config`).

| Variable | Default | Description |
| --- | --- | --- |
| `TORAN_SOCKET_PATH` | `/tmp/toran.sock` | Unix socket for the Python SDK. |
| `TORAN_API_BIND` | `127.0.0.1:7878` | HTTP address for the dashboard and webhooks. |
| `TORAN_POLICY_DIR` | `./policies` | Directory of YAML policies (hot-reloaded). |
| `TORAN_DATABASE_PATH` | `./toran.db` | SQLite database file (WAL mode). |
| `TORAN_DEFAULT_ACTION` | `ALLOW` | What to return when no rule matches. |
| `TORAN_MAX_CONNECTIONS` | `10000` | Concurrent socket connections. |
| `TORAN_MAX_SUSPENDED` | `10000` | Concurrent pending approvals. |
| `TORAN_DEFAULT_TIMEOUT` | `300` | Seconds before a pending approval times out. |
| `TORAN_HMAC_SECRET` | `change-me-in-production` | Secret for webhook signatures. |
| `TORAN_LOG_LEVEL` | `info` | `tracing` filter (`debug`, `info`, `warn`, `error`). |
| `TORAN_SLACK_WEBHOOK` | _(none)_ | Slack incoming-webhook URL. |
| `TORAN_GENERIC_WEBHOOK` | _(none)_ | Generic HMAC-signed webhook. |

> ⚠️ `fail_open` on the Python side defaults to **false**. If the core is unreachable, calls raise `ToranConnectionError` so the agent can fall back to safe mode.

---

## 🚢 Deployment

### 📦 Single binary (default)

```bash
./toran start
```

Everything runs in one process. SQLite for state, Unix socket for the SDK, HTTP for the dashboard.

### 🐳 Docker

A multi-stage [`Dockerfile`](./Dockerfile) (distroless runtime image) is included:

```bash
docker build -t toran .
docker run -p 7878:7878 -v "$PWD/data:/data" toran
```

The container binds the API to `0.0.0.0:7878` and persists state in the
`/data` volume.

### 🌐 Reverse proxy (TLS)

```nginx
location / {
    proxy_pass http://127.0.0.1:7878;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
}
```

---

## 🔐 Security model

- 🚫 **No code execution in policies.** Conditions are a fixed operator set: `eq`, `ne`, `contains`, `starts_with`, `ends_with`, `regex`, `gt`, `lt`, `gte`, `lte`, `in`, `not_in`, `exists`. No `eval`.
- 🎲 **Random notify tokens.** 16 bytes of CSPRNG, hex-encoded, attached to every approval record. The token is the only credential needed to resolve the approval.
- ⏱️ **Constant-time comparison.** Tokens are compared with a constant-time check, not `==`.
- ✍️ **HMAC signatures.** Webhook payloads carry an HMAC-SHA256 signature in the body for the receiver to verify.
- 📝 **Append-only audit log.** Records are only inserted. The schema has no `UPDATE` statement against existing audit rows.
- 🔗 **Tamper-evident chaining.** Each audit row can optionally chain `SHA-256(prev_hash || row_json)`.
- 🔒 **Socket permissions.** The Unix socket is created with mode `0660`. The default DB file is `0600`.

See [`specs/11-SECURITY.md`](./specs/11-SECURITY.md) for the full threat model and mitigations.

---

## ⌨️ CLI reference

```
$ toran --help
Runtime human-approval gatekeeper for AI agents

Usage: toran [OPTIONS] <COMMAND>

Commands:
  start     Start the socket server and REST API (default mode)
  validate  Validate all YAML policy files without starting the server
  status    Print a one-line status summary
  list      List pending or recent approvals
  approve   Approve a pending approval by id
  deny      Deny a pending approval by id
  help      Print this message or the help of the given subcommand(s)

Options:
      --config <CONFIG>  Path to a TOML config file (overrides env)
  -h, --help             Print help
  -V, --version          Print version
```

Examples:

```bash
toran validate
toran list --status pending --limit 20
toran approve <id> --by alice --token <notify_token>
toran deny <id> --by alice
toran status
```

---

## 🧪 Testing & benchmarks

```bash
# 🦀 Rust — 25 tests (10 policy, 7 state, 3 e2e, plus unit)
cargo test --all-features

# 🐍 Python — 6 decorator tests
cd sdk && python3 -m pytest -v

# 📈 Criterion benchmark (policy evaluation)
cargo bench --bench policy_eval
```

CI runs formatting (`cargo fmt --check`), linting (`cargo clippy -D warnings`), and the full test suite on every push and PR — see [`.github/workflows/ci.yml`](./.github/workflows/ci.yml).

---

## 🤝 Contributing

See [`CONTRIBUTING.md`](./CONTRIBUTING.md) for the full guide. The short version:

- `cargo fmt` before every commit.
- `cargo clippy -- -D warnings` must pass.
- Add tests for new features.
- Update [`CHANGELOG.md`](./CHANGELOG.md).
- Use conventional commits: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `security:`.

Dev loop:

```bash
cargo watch -x check -x clippy -x test
cargo run -- start
# in another shell
TORAN_SOCKET_PATH=/tmp/toran.sock python3 sdk/examples/minimal.py
```

---

## 📄 License

[MIT](./LICENSE) © [Zyferon](https://github.com/Zyferon)

---

<div align="center">

Maintained by [**Zyferon**](https://github.com/Zyferon) · [Report an issue](https://github.com/Zyferon/toran/issues)

</div>
