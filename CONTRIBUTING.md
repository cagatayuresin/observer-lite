# Contributing to Observer Lite

Thank you for taking the time to contribute! Observer Lite is a self-hosted uptime monitoring tool built with Python/FastAPI and Vue 3. Contributions of all kinds are welcome — bug fixes, new features, documentation improvements, and test coverage.

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

### Backend (Python 3.12+)

```bash
cd backend

# Create a virtual environment (uv is recommended, pip works too)
python3.12 -m venv .venv
source .venv/bin/activate

# Install dependencies in editable mode
pip install -e ".[dev]"

# Apply database migrations
DATABASE_URL="sqlite+aiosqlite:////tmp/observer_dev.db" \
DATABASE_PATH="/tmp/observer_dev.db" \
alembic upgrade head

# Start the backend
DATABASE_URL="sqlite+aiosqlite:////tmp/observer_dev.db" \
DATABASE_PATH="/tmp/observer_dev.db" \
SECRET_KEY="dev-secret-do-not-use-in-prod" \
PORT=8080 \
uvicorn app.main:app --host 127.0.0.1 --port 8080 --reload --workers 1
```

> **`--workers 1` is required.** APScheduler and the SSE broadcaster are in-process; multiple workers cause duplicate checks and missed events.

### Frontend (Node 20+)

```bash
cd frontend
npm install
npm run dev   # Vite dev server at http://localhost:5173
              # API proxied to http://localhost:8080
```

Open **http://localhost:5173** in your browser. Default credentials: `admin` / `admin`. You will be prompted to change the password on first login.

---

## Project Structure

```
observer-lite/
├── backend/
│   ├── app/
│   │   ├── checkers/      # Monitor type implementations (HTTP, Ping, SSL, Heartbeat)
│   │   ├── db/            # SQLAlchemy models and async session factory
│   │   ├── routers/       # FastAPI routers (one per resource)
│   │   ├── schemas/       # Pydantic v2 request/response schemas
│   │   ├── services/      # Business logic (check processing, notifications, stats)
│   │   ├── scheduler/     # APScheduler engine, job manager, and job functions
│   │   ├── sse/           # Server-Sent Events broadcaster
│   │   └── utils/         # Crypto helpers, pagination
│   ├── alembic/           # Database migrations
│   └── tests/             # pytest test suite
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

### Python

- **Style:** Follow [PEP 8](https://peps.python.org/pep-0008/). We use `ruff` for linting and formatting.
  ```bash
  cd backend
  ruff check app/
  ruff format app/
  ```
- **Type hints:** All public functions must have full type annotations.
- **Docstrings:** Use [Google-style docstrings](https://google.github.io/styleguide/pyguide.html#383-functions-and-methods) for all public modules, classes, and functions.
- **Async:** All DB operations and external I/O must be `async`. Never call blocking code on the event loop without `asyncio.to_thread`.
- **Security:** Never log request bodies, passwords, or tokens. Validate all user-controlled input with Pydantic.

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
cd backend

# Run the full test suite with coverage
.venv/bin/python -m pytest tests/ --cov=app --cov-report=term-missing -v

# Run a specific test file
.venv/bin/python -m pytest tests/test_matchers.py -v
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
