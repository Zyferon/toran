# CONTRIBUTING GUIDE
## How to Help Build Toran

### Development Setup

#### Prerequisites
- Rust 1.80+ (install via rustup.rs)
- Python 3.10+ (install via pyenv or python.org)
- Node.js 20+ (install via nvm or nodejs.org)
- Git

#### Step 1: Clone the Repository
```bash
git clone https://github.com/Zyferon/toran.git
cd toran
```

#### Step 2: Install Rust Dependencies
```bash
cd src/core
cargo build
```

#### Step 3: Install Python Dependencies
```bash
cd src/sdk
python -m venv .venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate
pip install -e ".[dev]"
```

#### Step 4: Install Dashboard Dependencies
```bash
cd src/dashboard
npm install
```

#### Step 5: Run the Development Stack
```bash
# Terminal 1: Rust core
cd src/core
cargo run -- --config ../../configs/dev.toml

# Terminal 2: Dashboard
cd src/dashboard
npm run dev

# Terminal 3: Python SDK tests
cd src/sdk
pytest
```

### Code Standards

#### Rust
- Use `cargo fmt` before committing. The CI will reject unformatted code.
- Use `cargo clippy` and fix all warnings. The CI will reject code with warnings.
- Write doc comments for all public functions and structs (`///`).
- Use `anyhow` for error handling in binaries, `thiserror` for libraries.
- Prefer `async`/`await` over callbacks. Use Tokio for all async code.
- Avoid `unsafe` blocks. If you must use `unsafe`, document why and get a second review.
- Write unit tests for all pure functions. Write integration tests for all I/O.

#### Python
- Use `black` for formatting, `ruff` for linting, `mypy` for type checking.
- Write docstrings for all public functions (Google style).
- Use type hints everywhere. No bare `def foo(bar)` without types.
- Use `pytest` for tests. Aim for 80% coverage minimum.
- Use `pydantic` for configuration validation and data models.
- Avoid `eval()`, `exec()`, or dynamic code execution. Security is paramount.

#### TypeScript / Next.js
- Use `prettier` for formatting, `eslint` for linting.
- Use TypeScript strict mode. No `any` types without justification.
- Use React Server Components by default. Use Client Components only for interactivity.
- Use `shadcn/ui` components. Do not write custom CSS when a shadcn component exists.
- Write Playwright tests for all user flows (approval, denial, navigation).

### Git Workflow

#### Branch Naming
- `feature/description` — New features
- `bugfix/description` — Bug fixes
- `docs/description` — Documentation changes
- `refactor/description` — Code refactoring
- `security/description` — Security fixes

#### Commit Messages
Use conventional commits:
```
feat: add Slack notification adapter
fix: handle timeout in async wait
docs: update self-hosting guide
refactor: simplify decision tree traversal
security: add HMAC verification to webhooks
```

#### Pull Request Process
1. Fork the repository (or create a branch if you have write access).
2. Write your code with tests.
3. Run the full test suite locally (`make test`).
4. Open a PR with a clear description of what changed and why.
5. Link to any related issues.
6. Wait for CI to pass.
7. Wait for one approval from a maintainer.
8. Squash and merge.

### Testing

#### Running Tests
```bash
# Rust tests
cd src/core
cargo test

# Python tests
cd src/sdk
pytest

# Dashboard tests
cd src/dashboard
npm run test

# End-to-end tests
cd src/dashboard
npm run test:e2e
```

#### Writing Tests
- Test the happy path, the error path, and the edge case.
- Use property-based testing (proptest in Rust, Hypothesis in Python) for complex logic.
- Mock external services (Slack API, email servers) in unit tests. Use real services in integration tests only.
- Benchmark before and after performance-critical changes.

### Reporting Security Issues

Do NOT open a public issue for security vulnerabilities. Report them privately via
[GitHub Security Advisories](https://github.com/Zyferon/toran/security/advisories/new) with:
- A description of the vulnerability
- Steps to reproduce
- The impact (what could an attacker do?)
- Your PGP key (optional)

We will respond within 48 hours and coordinate a disclosure timeline.

### Getting Help

- Open a [GitHub Discussion](https://github.com/Zyferon/toran/discussions) for architecture questions
- Open a [GitHub Issue](https://github.com/Zyferon/toran/issues) for bug reports and feature requests

### Code of Conduct

Be respectful. Be constructive. Assume good intent. No harassment, discrimination, or toxicity. We are building infrastructure for the future. Act like it.
