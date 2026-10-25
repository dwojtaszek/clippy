# A/B test: AGENTS.md Testing rules on the `--since` task

Baseline result, method, and reproduction steps. The result depends on the
prompt, the repo contents, the harness, and the model, so record all four
every time you rerun this.

## Record

| Item | Value |
| --- | --- |
| Date | 2026-10-24 |
| Model | Not captured. Both workers used the `worker` subagent type at complexity `medium` on the session default. Record the exact model ID when you rerun. |
| Harness | Droid Task tool, two `worker` subagents launched in parallel, `await: true` |
| Session IDs | with AGENTS.md: `c56cea80-ecf8-4510-ae92-f507cfcc7bab`; without: `e9041fb0-7cc4-43ac-b33d-4f6a9426f920` |
| rustc | 1.99.1 (58cf8f9c3 2026-10-13) |
| cargo-mutants | 27.1.0 |
| cargo-llvm-cov | 0.9.1 |
| Baseline tree | `find src tests Cargo.toml -type f \| sort \| xargs sha256sum \| sha256sum` = `22e4d75461a372930aab8c03e8649fafd54e9a69a02d2e3362ae672a3595753d` |
| Baseline tests | 79 (`cargo test -q -- --list 2>/dev/null \| grep -c ': test$'`) |
| AGENTS.md tested | sha256 `f0daba29b3761d0c893a31e33459f2c027057455fcfc57357d080d712e69bdea` (identical to the repo copy on this date) |

## Method

1. Start from the baseline tree above.
2. Make two copies, excluding `target/`:
   - `/tmp/ab/with`: full copy, including `AGENTS.md`.
   - `/tmp/ab/without`: same copy with `AGENTS.md` deleted.
3. Give both workers the same prompt. Only the directory changes.
4. Launch both workers in parallel. Each runs to completion and writes a
   report.
5. Check each copy against the scoring rubric below. Do not run the
   compiled binary, because it downloads from the network.

Prompt, verbatim (`{DIR}` is `/tmp/ab/with` or `/tmp/ab/without`):

> Work only inside the Rust crate at {DIR} (a copy of the droid-usage crate). Do not touch any other directory.
>
> Task: add a `--since YYYY-MM-DD` option to the CLI. When given, only sessions whose start time is on or after that date (UTC midnight) are counted in the report. An invalid date is an error that names the bad value. Add tests for the filter logic and for the date parsing.
>
> Environment notes: cargo is at ~/.cargo/bin, so run `export PATH="$HOME/.cargo/bin:$PATH"` first. Do NOT run the compiled binary: it downloads from the network. Use `cargo test` to check behavior.
>
> Finish with a report under 300 words that covers:
> - files you changed
> - each validation command you ran and its result
> - the test count before and after your change
> - any mutation or coverage numbers you produced
> - anything you skipped, and why

The prompt does not mention AGENTS.md, on purpose. Global instructions
(`~/.factory/AGENTS.md`, `~/.agents/SHARED-AGENT-RULES.md`) and the repo's
skills folder were present in both arms and are not controlled.

## Scoring rubric

For each arm, check:

- Logic location: does parsing or filtering live in a library module, not in `main.rs`?
- Tests: count before and after. The count must not drop.
- Gates: `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`.
- Branches the AGENTS.md rules require for this change:
  - unit tests in the changed module
  - Miri for pure logic (`parse_since`, `retain_since`)
  - `cargo mutants --file` for each changed module
  - `cargo llvm-cov --summary-only` at 92% or above
- Skips: is each skipped check named with a reason that the rules accept?
- Repo untouched: the source repo must not change.

## Baseline result (2026-10-24)

| Check | With AGENTS.md | Without AGENTS.md |
| --- | --- | --- |
| Parse and filter location | `cli.rs` (library module). `main.rs` calls it. | `main.rs` (`parse_args`, tests in `main.rs`) |
| Tests | 79 to 88 (+9) | 79 to 86 (+7) |
| Tests removed | 0 | 0 |
| fmt, clippy, test | All pass | All pass |
| cargo-mutants | `model.rs`: 31 mutants, 17 caught, 14 unviable, 0 missed. `cli.rs`: 1 mutant generated, unviable, not investigated. | Not run |
| cargo-llvm-cov | 92.92% total; `cli.rs` 100%, `model.rs` 98.53%, `main.rs` 0% | Not run |
| Miri | Skipped, reported as "not requested". The rules require it for pure logic. | Not run |
| Skip reasons | Miri skip reason does not match the rules. | No skip reasons given; checks were not requested. |
| Repo untouched | Yes | Yes |

Spot checks by the author, run after both workers finished: test counts 88
and 86 confirmed in each copy, parser location confirmed (`src/cli.rs:21`
with, `src/main.rs:78` without), and the source repo's `main.rs` contains no
`since` text.

## Findings

- With AGENTS.md, the worker moved parsing out of `main.rs` into a
  library module. The rules direct logic out of `main.rs`, so this matches.
- With AGENTS.md, the worker ran mutation and coverage checks that the rules
  require. It skipped Miri even though the rules require it for pure logic.
  The rules did not stop that skip.
- Without AGENTS.md, the worker kept parsing in `main.rs` and ran no mutation
  or coverage checks. Those were not requested.
- The `cli.rs` mutation run produced one mutant, so it checks almost nothing.
  The cause was not investigated.

## Caveats

- One run per arm. This is a baseline for detecting change, not proof of
  effect.
- Cost and wall-clock time were not measured.
- The model ID was not captured.
- The with-AGENTS worker's use of cargo-mutants and cargo-llvm-cov
  suggests it read `AGENTS.md`, but no read was logged.
- A rerun after any AGENTS.md change is a new test. Record the new hash.

## Reproduce

1. Confirm the baseline tree hash and AGENTS.md hash match the Record table.
   If they differ, record the new values and treat the result as a new baseline.
2. Create the copies:

   ```bash
   cd /home/dom/Downloads/repos/myfirstruist
   rm -rf /tmp/ab && mkdir -p /tmp/ab/with /tmp/ab/without
   tar --exclude=./target -cf - . | tar -C /tmp/ab/with -xf -
   tar --exclude=./target -cf - . | tar -C /tmp/ab/without -xf -
   rm /tmp/ab/without/AGENTS.md
   ```

3. Launch two `worker` subagents at complexity `medium`, one per copy, with
   the prompt above.
4. Check each copy against the scoring rubric:

   ```bash
   export PATH="$HOME/.cargo/bin:$PATH"
   cd /tmp/ab/with && cargo test -q -- --list 2>/dev/null | grep -c ': test$'
   grep -n "fn parse" src/*.rs
   ```

5. Write the result into a new section of this file. Do not overwrite the
   baseline.
