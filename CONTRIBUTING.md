# Contributing to Observer Lite

Thank you for taking the time to contribute! Observer Lite is a self-hosted uptime monitoring tool built with Rust and Vue 3. Contributions of all kinds are welcome — bug fixes, new features, documentation improvements, and test coverage.

## Table of Contents

- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Coding Standards](#coding-standards)
- [Running Tests](#running-tests)
- [Submitting a Pull Request](#submitting-a-pull-request)
- [Reporting Bugs](#reporting-bugs)
- [Feature Requests](#feature-requests)

---

## Getting Started

1. **Fork** the repository on GitHub.
2. **Clone** your fork locally:
   ```bash
   git clone https://github.com/<your-username>/observer-lite.git
   cd observer-lite
   ```
3. Create a feature branch:
   ```bash
   git checkout -b feat/your-feature-name
   ```

---

## Development Setup

### Server (Rust)

```bash
cd frontend && npm ci && npm run build && cd ..
SECRET_KEY="dev-secret-do-not-use-in-prod" \
DATABASE_PATH="./observer.db" \
cargo run
```

The process listens on port 3000. Run one process per data directory. The scheduler lives in that process.

### Frontend (Node 22+)

```bash
cd frontend
npm ci
npm run dev   # Vite dev server at http://localhost:5173
              # API proxied to http://localhost:3000
```

Open **http://localhost:5173** in your browser. Default credentials: `admin` / `admin`. You will be prompted to change the password on first login.

---

## Project Structure

```
observer-lite/
├── crates/observer/
│   ├── src/checkers/      # HTTP, ping, SSL, heartbeat
│   ├── src/routes/        # HTTP API
│   ├── src/services/      # Check processing, notifications, retention
│   └── tests/             # API tests
└── frontend/
    └── src/
        ├── api/           # Axios API clients
        ├── components/    # Reusable Vue components
        ├── pages/         # Route-level page components
        ├── router/        # Vue Router configuration and guards
        └── stores/        # Pinia state stores
```

---

## Coding Standards

### Rust

- Format with `cargo fmt` and keep `cargo clippy --workspace --all-targets -- -D warnings` clean.
- Do not log passwords, tokens, or request bodies.
- Blocking work (bcrypt, ICMP) goes through `spawn_blocking`.

### TypeScript / Vue

- **Style:** ESLint + Prettier (configured in the project).
  ```bash
  cd frontend
  npm run lint
  npm run type-check
  ```
- **Components:** Use `<script setup lang="ts">` (Composition API). No Options API.
- **State:** Global state belongs in a Pinia store. Component-local state uses `ref`/`reactive`.
- **No CDN dependencies.** All assets must be bundled (no `<link>` or `<script>` tags pointing to external URLs).

---

## Running Tests

```bash
cargo test --workspace
cargo llvm-cov --workspace --fail-under-lines 80
```

Coverage must remain **above 80%**. PRs that reduce coverage below this threshold will not be merged.

For frontend type checking:

```bash
cd frontend
npm run type-check
npm run build   # also validates the build
```

---

## Submitting a Pull Request

1. **Ensure tests pass** and coverage stays above 80%.
2. **Add or update tests** for any logic you change.
3. **Update docstrings** for any functions you add or modify.
4. **Keep commits focused.** One logical change per commit. Write clear commit messages:
   ```
   feat(checkers): add DNS resolution checker type
   fix(scheduler): prevent duplicate jobs on monitor update
   docs(contributing): add setup instructions for Windows
   ```
5. **Open the PR** against the `main` branch.
6. Fill in the PR description template — describe what changed and why, and include a test plan.

### Commit Message Format

We follow [Conventional Commits](https://www.conventionalcommits.org/):

| Prefix | Use for |
|--------|---------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `test` | Adding or fixing tests |
| `refactor` | Code change without feature or fix |
| `chore` | Maintenance (deps, build, CI) |
| `perf` | Performance improvement |

---

## Reporting Bugs

Use [GitHub Issues](https://github.com/cagatayuresin/observer-lite/issues) with the **bug** label. Include:

- Observer Lite version (see `VERSION` file)
- Deployment method (Docker / bare metal)
- Steps to reproduce
- Expected vs. actual behaviour
- Relevant logs (`docker logs <container>`)

---

## Feature Requests

Open a [GitHub Issue](https://github.com/cagatayuresin/observer-lite/issues) with the **enhancement** label. Describe:

- The problem you are trying to solve
- Your proposed solution
- Any alternatives you have considered

Large features should be discussed in an issue before implementation to avoid wasted effort.

---

## Code of Conduct

Be respectful and constructive. We follow the [Contributor Covenant](https://www.contributor-covenant.org/) Code of Conduct. Harassment or discrimination of any kind will not be tolerated.
