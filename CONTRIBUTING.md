# Contributing to Toran

Thanks for your interest. This project is built by a small team and
we welcome focused, well-tested contributions.

## Development setup

Prerequisites:

- Rust 1.85+ (`rustup update stable`)
- Python 3.10+
- `cargo-watch` for the dev loop (`cargo install cargo-watch`)
- `sqlx-cli` is **not** required (we use `rusqlite` with bundled SQLite)

Clone and build:

```bash
git clone https://github.com/Zyferon/toran.git
cd toran
cargo build
```

Install Python dev dependencies:

```bash
python3 -m pip install -e ".[dev]"
```

## Dev loop

```bash
cargo watch -x check -x clippy -x test
# in a second shell
RUST_LOG=debug cargo run -- start
# in a third shell
TORAN_SOCKET_PATH=/tmp/toran.sock python3 sdk/examples/minimal.py
```

## Code standards

### Rust

- `cargo fmt` before every commit.
- `cargo clippy -- -D warnings` must pass. The crate has
  `#![deny(warnings)]` and `#![deny(clippy::all)]`.
- Doc comments (`///`) on every public item.
- `anyhow::Result` for binaries, `thiserror` for library error
  types.
- `tokio` for async, `parking_lot` for locks.
- No `unsafe` unless absolutely necessary; explain why in a comment
  and get a second review.

### Python

- Type hints on every public function.
- `ruff check .` and `ruff format .` for linting and formatting.
- `mypy` for type checking (best-effort; we don't run it in CI yet).
- Use the bundled `Client`; don't open new sockets.
- Never block the event loop inside an async wrapper.

### YAML policies

- One file = one policy. Filename matches `name:` for traceability.
- Lowercase rule names. Use kebab-case for multi-word names.
- `description:` is required for any non-trivial rule.

## Branch and commit conventions

Branches: `feat/...`, `fix/...`, `docs/...`, `refactor/...`,
`test/...`, `security/...`.

Commits: [Conventional Commits](https://www.conventionalcommits.org/).

```
feat: add CrewAI integration wrapper
fix: handle WebSocket close without leaking the task
docs: document the toran_signature webhook header
refactor: split evaluator into lookup and compare
security: add constant-time token comparison
test: cover fallback action when no rule matches
```

## Testing

```bash
# Rust
cargo test
cargo test --release              # for benchmark-related code
cargo bench --bench policy_eval   # for perf-sensitive changes

# Python
cd sdk && python3 -m pytest -v
```

Write a test for every new feature. The CI runs the same commands on
every PR.

## Security disclosures

Please **do not** open a public issue for security vulnerabilities. Instead,
report them privately via [GitHub Security Advisories](https://github.com/Zyferon/toran/security/advisories/new)
with:

- A description of the vulnerability.
- Steps to reproduce.
- The impact (what could an attacker do?).

We aim to acknowledge within 48 hours.

## Code of conduct

Be respectful. Be constructive. Assume good intent. We are building
infrastructure for the future. Act like it.
