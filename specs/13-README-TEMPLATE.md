# README TEMPLATE
## Copy This, Customize It, Ship It

```markdown
# Toran

**Runtime human approval gates for AI agents. Framework-agnostic. Sub-millisecond. Self-hosted.**

[Demo Video](link) | [Documentation](link) | [Discord](link) | [MIT License](link)

---

## What is Toran?

Toran is a gatekeeper that sits between your AI agent and the real world. When your agent tries to send an email, write to a database, or call an API, Toran checks a policy. If the action is allowed, it executes immediately. If it is risky, Toran pauses the agent and asks a human for approval.

Toran works with any Python agent framework — LangChain, CrewAI, Pydantic AI, AutoGen, or a simple `for` loop. One decorator. No rewrite.

## Why Toran?

| | Toran | LangGraph HITL | Langfuse | Braintrust |
|---|---|---|---|---|
| **Framework lock-in** | None | LangGraph only | None | None |
| **Runtime blocking** | Yes | Yes | No (observes only) | No (observes only) |
| **Policy as code** | YAML | Python dict | N/A | N/A |
| **Self-hosted** | Default | N/A | Self-hosted option | Cloud only |
| **Latency** | <1ms | ~5ms | N/A | N/A |
| **Open source** | MIT | Apache 2.0 | MIT | Proprietary |

## 30-Second Demo

```python
from toran import gate

@gate()
def send_email(to, subject, body):
    return mailgun.send(to, subject, body)

# Agent tries to send an email
send_email("boss@company.com", "Wire Transfer", "Send $50,000 to...")
# → Toran pauses execution
# → Slack notification: "Approve this email?"
# → You click "Approve"
# → Email sends
```

## Installation

```bash
pip install toran
```

Or download the standalone binary:
```bash
curl -sSL https://toran.dev/install.sh | bash
```

## Quick Start

1. **Write a policy** (`policies/email.yaml`):
```yaml
rules:
  - tool: send_email
    action: require_approval
    timeout: 300
```

2. **Decorate your function**:
```python
from toran import gate

@gate(policy="email")
def send_email(to, subject, body):
    return mailgun.send(to, subject, body)
```

3. **Run your agent**:
```python
agent.run()  # When send_email is called, Toran pauses and asks for approval
```

4. **Approve via Slack** or the web dashboard.

See the [full documentation](link) for framework integrations, notification setup, and deployment guides.

## Features

- **Framework-agnostic**: Works with LangChain, CrewAI, Pydantic AI, AutoGen, or custom Python.
- **Sub-millisecond**: Policy evaluation in Rust. No perceptible delay for allowed actions.
- **Human-in-the-loop**: Slack, email, Discord, or custom webhooks for approval requests.
- **Policy as code**: YAML policies in version control. Reviewable. Diffable. Auditable.
- **Self-hosted by default**: The core runs on your hardware. No required cloud service.
- **Audit trails**: Every decision logged. Every approval attributed. Compliance-ready.

## Documentation

- [Quick Start](link)
- [Policies](link)
- [Framework Integrations](link)
- [Self-Hosting](link)
- [SDK Reference](link)
- [Contributing](link)

## Community

- [Discord](link) — Ask questions, share projects, get help
- [GitHub Discussions](link) — Feature requests, architecture discussions
- [Twitter/X](link) — Updates and announcements

## License

MIT. See [LICENSE](LICENSE).
```

### How to Customize This
1. Replace all `(link)` placeholders with real URLs after you set up the docs site and Discord.
2. Record the demo video and replace the `[Demo Video](link)` with an embedded GIF or Loom link.
3. Update the comparison table if competitor features change.
4. Add a "Benchmarks" section after you run the Criterion benchmarks.
5. Add a "Changelog" section linking to CHANGELOG.md.
6. Add a "Sponsors" section if you get GitHub Sponsors.
