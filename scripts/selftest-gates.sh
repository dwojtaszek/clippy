#!/usr/bin/env bash
# Checks the gate logic in common.sh against known inputs. It runs in a scratch copy with a stub cargo,
# so the real mutants.out and the real known-equivalent list are never touched.
# Usage: scripts/selftest-gates.sh

set -uo pipefail

scratch=$(mktemp -d) || exit 2
trap 'cd / && rm -rf "$scratch"' EXIT
mkdir -p "$scratch/scripts"
cp "$(dirname "$0")/common.sh" "$scratch/scripts/"
cd "$scratch" || exit 2

# Sourcing common.sh cds to the root of the copy, so mutants.out and the known list are scratch files.
# shellcheck source=/dev/null
source scripts/common.sh
printf '%s\n' "src/agy.rs:79:42: replace | with ^ in model_from_db" >scripts/known-equivalent-mutants.txt

failures=()

# check NAME WANT GOT: records a failure when GOT is not WANT.
check() {
    if [[ $2 == "$3" ]]; then
        echo "ok   $1"
    else
        echo "FAIL $1: want $2, got $3"
        failures+=("$1")
    fi
}

# Stands in for cargo. Each case writes the verdict files that cargo-mutants would write, then returns its exit status.
cargo() {
    mkdir -p mutants.out
    : >mutants.out/caught.txt
    : >mutants.out/missed.txt
    : >mutants.out/timeout.txt
    : >mutants.out/unviable.txt
    case $STUB in
        all_unviable)
            echo "src/x.rs:1:1: r" >mutants.out/unviable.txt
            return 0
            ;;
        caught_and_unviable)
            echo "src/x.rs:1:1: r" >mutants.out/caught.txt
            echo "src/y.rs:1:1: r" >mutants.out/unviable.txt
            return 0
            ;;
        nothing_found) return 0 ;;
        known_missed)
            echo "src/agy.rs:79:42: replace | with ^ in model_from_db" >mutants.out/missed.txt
            return 2
            ;;
        unknown_missed)
            echo "src/x.rs:1:1: r" >mutants.out/missed.txt
            return 2
            ;;
        timed_out)
            echo "src/x.rs:1:1: r" >mutants.out/timeout.txt
            return 3
            ;;
        stopped) return 4 ;;
    esac
}

# expect_verdict CASE WANT: runs run_mutants as cargo-mutants reports CASE, then checks its exit status.
expect_verdict() {
    STUB=$1 run_mutants --timeout 60 >/dev/null 2>&1
    check "mutant verdict: $1" "$2" "$?"
}

expect_verdict all_unviable 1
expect_verdict caught_and_unviable 0
expect_verdict nothing_found 0
expect_verdict known_missed 0
expect_verdict unknown_missed 1
expect_verdict timed_out 1
expect_verdict stopped 1

check "time limit, no mutants" 90 "$(time_limit 0)"
check "time limit, 32 mutants" 170 "$(time_limit 32)"
check "time limit, capped" 300 "$(time_limit 100)"

echo
if ((${#failures[@]})); then
    echo "FAILED: ${failures[*]}"
    exit 1
fi
echo "PASSED: every gate check matched"
