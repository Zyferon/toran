# Toran — Guided Tour

A 5-minute walkthrough of Toran for first-time users. Read this
end-to-end the first time; skim it on the second.

## What you are looking at

Toran is a runtime human-approval gatekeeper. Your AI agent calls
its normal functions. Behind the scenes, a decorator (`@gate` in
Python, or any SDK you wrap) routes the call through Toran. Toran
reads a YAML policy, picks one of three decisions
(`ALLOW`, `BLOCK`, `REQUIRE_APPROVAL`), and either runs the
function, blocks it, or pauses and asks a human.

Humans resolve the pause from any of these surfaces:

1. **Web dashboard** — `http://127.0.0.1:7878/dashboard`
2. **CLI** — `toran approve <id> --by you`
3. **HTTP API** — `POST /api/approvals/:id/{approve,deny}`
4. **Slack / webhook** — fan-out notifications

The dashboard is what most operators use day-to-day. Click
**Take the tour** in the nav to get a 4–6 step walkthrough on any
page. The tour auto-plays once per page (tracked in
`localStorage`).

## Concepts in 90 seconds

| Term | Meaning |
| --- | --- |
| **Policy** | A YAML file in `./policies/`. Lists rules, sets a default action. Hot-reloaded on save. |
| **Rule** | One match in a policy: `tool` (function name or glob), `conditions` (zero or more key/op/value checks), and an `action`. |
| **Action** | One of `ALLOW`, `BLOCK`, `REQUIRE_APPROVAL`. The return value of policy evaluation. |
| **Approval** | A row in SQLite that records a paused function call, its arguments, and its notify token. The Python SDK blocks on this row. |
| **Audit entry** | An append-only row in the `audit_log` table. Every decision, create, and resolve writes one. |
| **Notify token** | 32-hex-char secret attached to every approval. Required to approve/deny via CLI, dashboard, or webhook. Same secret is also the HMAC for webhooks. |

## Day-to-day workflow

### Step 1 — start the server

```bash
./target/release/toran start
#   → toran socket server listening socket=/tmp/toran.sock
#   → toran api listening addr=127.0.0.1:7878
```

Open `http://127.0.0.1:7878`. The home page shows live status.

### Step 2 — instrument your agent

```python
from toran import gate, configure

configure(socket_path="/tmp/toran.sock", agent_id="prod-agent-1")

@gate()
def send_email(to, subject, body):
    return mailgun.send(to, subject, body)
```

Every call to `send_email` is now routed through Toran.

### Step 3 — write or edit a policy

Open `policies/email-guardian.yaml` (or any of the four other
example policies in the directory). Change a `risk_score`, add a
condition, raise a `priority`. The server hot-reloads the file;
no restart needed.

### Step 4 — wait for a real call

When the agent hits a `REQUIRE_APPROVAL` rule, an approval row
appears in the dashboard. The Python SDK blocks at the
`@gate`-wrapped call site.

### Step 5 — resolve it

Three options:

```bash
# CLI (server must be running)
toran approve <id> --by you

# HTTP
curl -X POST http://127.0.0.1:7878/api/approvals/<id>/approve \
  -H 'content-type: application/json' \
  -d '{"resolved_by":"you","token":"<notify_token>"}'

# Web
open http://127.0.0.1:7878/dashboard/approval/<id>
```

The Python SDK wakes up and either runs the function
(`approve`) or raises `ToranDeniedError` (`deny`).

### Step 6 — read the audit log

The audit log is at `http://127.0.0.1:7878/dashboard/audit` and
the JSON form is at `/api/audit?limit=500`. Append-only SQLite.
Nothing ever edits a row.

## Common operations

### Add a new function to the gate

Drop `@gate()` on the function. Default action of every example
policy is `ALLOW`, so unconfigured calls pass. The first time you
want to require approval for a specific function, add a rule to
`policies/*.yaml`.

### Block a function outright

```yaml
- name: block-send-money
  tool: { exact: send_money }
  action: BLOCK
```

Risk score is informational; the action is what Toran actually
returns.

### Make a rule fire only sometimes

```yaml
- name: approve-big-transfers
  tool: { exact: send_money }
  conditions:
    - key: amount
      op: gt
      value: "1000"
  action: REQUIRE_APPROVAL
  timeout_secs: 300
```

Operators: `eq, ne, lt, le, gt, ge, in, not_in, contains, regex`.
Strings on the right, parsed by serde. The `regex` operator
compiles the value into a `regex::Regex` at policy load time.

### Send approvals to Slack

```bash
export TORAN_SLACK_WEBHOOK="https://hooks.slack.com/services/T0/B0/xxx"
./target/release/toran start
```

Slack message is Block-Kit format with `Approve`/`Deny` buttons
that link back to the dashboard.

### Send approvals to a custom webhook

```bash
export TORAN_GENERIC_WEBHOOK="https://my-app.example.com/toran"
export TORAN_HMAC_SECRET="$(openssl rand -hex 32)"
```

The body is JSON, with `X-Toran-Signature: <hmac-sha256-hex>`
header on the request, and the same signature embedded in the
body as `toran_signature`.

### Use the Python `@gate` decorator in tests

```python
import os
os.environ["TORAN_FAIL_OPEN"] = "1"  # allow everything if Toran is down
```

`TORAN_FAIL_OPEN=1` makes the SDK degrade to "allow" if the socket
is unreachable, so unit tests don't need a server. Never enable
this in production.

## Architecture in one paragraph

The Rust core is a Tokio binary. It binds a Unix-domain socket
(`/tmp/toran.sock` on Linux, `~/Library/Caches/...` on macOS) for
SDK clients, and a TCP port (`127.0.0.1:7878` by default) for the
REST API and dashboard. Policies are loaded from a directory
(default `./policies/`), compiled once, and held in an in-memory
`RwLock<PolicyStore>`. The `notify` crate watches the directory
and triggers a hot-reload on every save. The state manager is a
SQLite database in WAL mode (`./toran.db`). It holds the
`approvals` and `audit_log` tables. The Python SDK is a small
synchronous + asyncio client that opens the Unix socket, sends
length-prefixed JSON, and either gets a `Decision` back or a
`Wait` reply plus a poll loop that resolves when the human
clicks Approve/Deny.

## Where to look next

- `README.md` — full quickstart, env vars, deployment, security model
- `BUILD_REPORT.md` — every bug, every algorithm
- `policies/*.yaml` — 5 real-world examples
- `sdk/examples/` — minimal, LangChain, custom-framework
- `TOUR.md` — this file
