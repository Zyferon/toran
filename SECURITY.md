# Security Policy

## Supported versions

| Version | Supported |
| ------- | --------- |
| 0.1.x   | ✅        |

Toran is pre-1.0. Until a 1.0 release, only the latest `0.1.x` line
receives security fixes.

## Reporting a vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

Report privately via
[GitHub Security Advisories](https://github.com/Zyferon/toran/security/advisories/new).
Include:

- A description of the vulnerability.
- Steps to reproduce.
- The impact (what could an attacker do?).
- A suggested fix, if you have one.

We aim to acknowledge reports within **48 hours** and to coordinate a
disclosure timeline with you before any public release.

## Scope

Toran is a self-hosted gatekeeper that runs on your own infrastructure.
The areas most relevant to security review:

- **Policy evaluation** — confirm no rule can cause code execution; the
  operator set is fixed (`eq`, `ne`, `contains`, `starts_with`,
  `ends_with`, `regex`, `gt`, `lt`, `gte`, `lte`, `in`, `not_in`,
  `exists`) and there is no `eval`.
- **Approval tokens** — 128-bit CSPRNG tokens, compared in constant time.
- **Webhook signatures** — HMAC-SHA256 over the body using
  `TORAN_HMAC_SECRET`.
- **Audit log** — append-only, optionally hash-chained
  (`SHA-256(prev_hash || row_json)`).
- **Local IPC** — the Unix socket is created mode `0660`; the SQLite DB
  defaults to `0600`.

See [`specs/11-SECURITY.md`](./specs/11-SECURITY.md) for the full threat
model.

## Hardening checklist for operators

- [ ] Set a strong, unique `TORAN_HMAC_SECRET` (never ship the default
      `change-me-in-production`).
- [ ] Keep `TORAN_API_BIND` on `127.0.0.1` and front it with a TLS
      reverse proxy for any remote access.
- [ ] Run the core as a non-root user with least-privilege filesystem
      access to the socket and DB paths.
- [ ] Keep `fail_open=False` (the default) so the agent fails safe when
      the core is unreachable.
- [ ] Restrict who can read the SQLite DB — it contains approval
      arguments and tokens.
