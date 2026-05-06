---
title: Development
permalink: /development/
---

# Development

Observer Lite has a FastAPI backend and a Vue 3 frontend.

## Repository Layout

```text
observer-lite/
├── backend/
│   ├── app/
│   │   ├── checkers/
│   │   ├── db/
│   │   ├── routers/
│   │   ├── scheduler/
│   │   ├── services/
│   │   └── sse/
│   └── tests/
├── frontend/
│   └── src/
├── docs/
├── Dockerfile
└── docker-compose.yml
```

## Backend Setup

```bash
cd backend
uv venv
uv pip install -e ".[dev]"
alembic upgrade head
uvicorn app.main:app --reload --workers 1
```

The backend runs on:

```text
http://localhost:8000
```

## Frontend Setup

In another terminal:

```bash
cd frontend
npm install
npm run dev
```

The Vite dev server prints its local URL.

## Tests

Backend:

```bash
cd backend
DEBUG=false uv run pytest tests/ --cov=app --cov-report=term-missing --cov-fail-under=80
```

Frontend:

```bash
cd frontend
npm run type-check
npm run build
```

Lint:

```bash
cd backend
uv run ruff check app tests
```

## Documentation Site Locally

The docs are plain Jekyll-compatible Markdown plus a custom layout.

The GitHub workflow builds them automatically, but you can preview with a local Jekyll install:

```bash
cd docs
bundle init
bundle add github-pages
bundle exec jekyll serve --baseurl ""
```

Open:

```text
http://localhost:4000
```

Do not commit generated `_site/` output.

## Adding Backend Endpoints

Preferred flow:

1. Add or update a schema in `backend/app/schemas`.
2. Add router behavior in `backend/app/routers`.
3. Put business logic in `backend/app/services` when it is shared or stateful.
4. Add tests in `backend/tests`.
5. Update docs when the behavior is user-visible.

## Adding Monitor Logic

Checkers live in:

```text
backend/app/checkers/
```

Scheduler orchestration lives in:

```text
backend/app/scheduler/jobs.py
```

Result processing and incident state changes live in:

```text
backend/app/services/check_service.py
```

Keep checker functions focused on producing a `CheckResult`. Persisting results and opening incidents should stay in the service layer.

## Documentation Style

- Write user-facing docs in English.
- Prefer runnable examples over abstract descriptions.
- Keep secrets as placeholders.
- Mention whether an action is safe for production.
- Update troubleshooting when adding a failure mode.
