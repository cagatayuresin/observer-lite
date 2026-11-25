---
title: Development
permalink: /development/
---

# Development

Observer Lite is one Rust binary and a Vue 3 frontend. The binary embeds the built frontend, so a machine that only runs the image does not need Node or Python.

## Repository Layout

```text
observer-lite/
├── crates/observer/   # Axum server, checkers, scheduler, tests
├── frontend/src/      # Vue 3 UI
├── docs/
├── Dockerfile
└── docker-compose.yml
```

## Local setup

```bash
cd frontend
npm ci
npm run build
cd ..
SECRET_KEY=dev-secret DATABASE_PATH=./observer.db cargo run
```

The server listens on `http://localhost:3000`. For UI work, `npm run dev` in `frontend/` proxies `/api` to that port.

## Tests

```bash
cargo test --workspace
cd frontend && npm test && npm run type-check
```

`./scripts/analyze.sh` runs formatting, Clippy, tests, coverage, and dependency checks without pushing anything.

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

## Adding API behavior

1. Add the route in `crates/observer/src/routes`.
2. Keep shared check, notification, and retention logic in `crates/observer/src/services`.
3. Add a test in `crates/observer/tests` or next to the module.
4. Update the docs when the behavior is user-visible.

## Adding monitor logic

Probes live in `crates/observer/src/checkers`. The scheduler in `crates/observer/src/scheduler.rs` only reads the `monitors` table. Persisting a result and opening an incident stays in `crates/observer/src/services/checks.rs`. A checker returns a result; it does not write the database.

## Documentation Style

- Write user-facing docs in English.
- Prefer runnable examples over abstract descriptions.
- Keep secrets as placeholders.
- Mention whether an action is safe for production.
- Update troubleshooting when adding a failure mode.
