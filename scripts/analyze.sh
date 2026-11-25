#!/usr/bin/env bash
# Local quality gate. Nothing here pushes code or an image.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$root/target}"

echo "== rustfmt =="
cargo fmt --all -- --check

echo "== clippy =="
cargo clippy --workspace --all-targets -- -D warnings

echo "== test =="
cargo test --workspace

echo "== coverage =="
cargo llvm-cov --workspace --fail-under-lines 80

echo "== cargo audit =="
cargo audit

echo "== cargo deny =="
cargo deny check

echo "== frontend =="
(
  cd frontend
  npm ci --ignore-scripts
  npm test
  npm run type-check
  npm run build
  npm audit --omit=dev --audit-level=high
)

if [ -n "${SONAR_TOKEN:-}" ]; then
  echo "== sonar =="
  cargo clippy --workspace --message-format=json -- -D warnings > clippy.json || true
  if command -v cargo-llvm-cov >/dev/null 2>&1; then
    cargo llvm-cov --workspace --lcov --output-path lcov.info
  fi
  if command -v sonar-scanner >/dev/null 2>&1; then
    sonar-scanner
  elif command -v cargo-sonar-scanner >/dev/null 2>&1; then
    cargo sonar-scanner
  else
    echo "SONAR_TOKEN is set but neither sonar-scanner nor cargo-sonar-scanner is installed."
    exit 1
  fi
else
  echo "SONAR_TOKEN is unset. Clippy, tests, and dependency checks ran locally; SonarQube Cloud upload was skipped."
fi
