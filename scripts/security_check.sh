#!/usr/bin/env bash
# security_check.sh — run the same security checks as .github/workflows/security.yml locally.
#
# Usage: ./scripts/security_check.sh
# Tools that aren't installed are skipped with a warning instead of failing,
# so the script is useful even on a partially-provisioned machine.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FAILED=0

section() { printf '\n\033[1m== %s ==\033[0m\n' "$1"; }
skip()    { printf '\033[33mSKIP:\033[0m %s\n' "$1"; }
fail()    { printf '\033[31mFAIL:\033[0m %s\n' "$1"; FAILED=1; }
ok()      { printf '\033[32mOK:\033[0m %s\n' "$1"; }

section "Secret scanning (gitleaks)"
if command -v gitleaks >/dev/null 2>&1; then
    if gitleaks detect --source "$ROOT" --no-banner; then ok "no secrets found"; else fail "gitleaks found leaks"; fi
else
    skip "gitleaks not installed (https://github.com/gitleaks/gitleaks)"
fi

section "Rust dependency audit (cargo audit)"
if command -v cargo-audit >/dev/null 2>&1 || cargo audit --version >/dev/null 2>&1; then
    if (cd "$ROOT/backend" && cargo audit); then ok "no known CVEs in Cargo.lock"; else fail "cargo audit reported vulnerabilities"; fi
else
    skip "cargo-audit not installed (cargo install cargo-audit)"
fi

section "Node dependency audit (npm audit)"
if command -v npm >/dev/null 2>&1 && [ -d "$ROOT/frontend/node_modules" ]; then
    if (cd "$ROOT/frontend" && npm audit --audit-level=high); then ok "no high/critical advisories"; else fail "npm audit reported high/critical advisories"; fi
else
    skip "npm not installed or frontend/node_modules missing (run npm install first)"
fi

section "Python dependency audit (pip-audit)"
if command -v pip-audit >/dev/null 2>&1; then
    if pip-audit -r "$ROOT/requirements.txt"; then ok "no known CVEs in requirements.txt"; else fail "pip-audit reported vulnerabilities"; fi
elif command -v pipx >/dev/null 2>&1; then
    if pipx run pip-audit -r "$ROOT/requirements.txt"; then ok "no known CVEs in requirements.txt"; else fail "pip-audit reported vulnerabilities"; fi
else
    skip "pip-audit not installed (pip install pip-audit)"
fi

section "Result"
if [ "$FAILED" -ne 0 ]; then
    echo "One or more security checks FAILED."
    exit 1
fi
echo "All executed security checks passed."
