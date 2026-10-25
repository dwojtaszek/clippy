#!/usr/bin/env bash
# Gate before you push or update a pull request. Runs every test and the lint and doc checks,
# and mutates only the lines that differ from BASE. Hard limit: 120 seconds.
# Usage: scripts/pre-pr-test.sh [BASE]   BASE defaults to origin/main.
# DRY_RUN=1 prints the plan without running it.

# Enforce the 120-second limit by running this same script under timeout.
if [[ -z ${PRE_PR_TEST_TIMED:-} ]]; then
    PRE_PR_TEST_TIMED=1 timeout --kill-after=10 120 "$0" "$@"
    code=$?
    if ((code == 124 || code == 137)); then
        echo "pre-pr-test: stopped at the 120-second limit before it finished; the gate did not pass" >&2
    fi
    exit "$code"
fi

# shellcheck source=scripts/common.sh
source "$(dirname "$0")/common.sh"

base=${1:-origin/main}
if ! git rev-parse --verify --quiet "$base^{commit}" >/dev/null; then
    echo "pre-pr-test: unknown base $base (run git fetch first)" >&2
    exit 2
fi

changed=$(git diff --name-only "$base")

run_step "clean tree" check_clean_tree
run_step "fmt" cargo fmt --check
run_step "clippy" cargo clippy --all-targets --all-features -- -D warnings
run_step "tests" cargo test -q
run_step "test count" check_test_count
run_step "docs" check_docs
if grep -qxE 'Cargo\.toml|Cargo\.lock|deny\.toml' <<<"$changed"; then
    run_step "cargo deny" cargo deny check
fi
if grep -qxE 'src/(prices|multipliers)\.rs' <<<"$changed"; then
    run_step "price keys in newest snapshot" python3 scripts/check-price-keys.py
fi
if grep -qE '^src/.*\.rs$' <<<"$changed"; then
    run_step "mutants on changed lines" changed_lines_mutants "$base"
fi
finish
