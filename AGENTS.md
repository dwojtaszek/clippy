# Project agent rules

Read `/home/dom/.agents/SHARED-AGENT-RULES.md` before coding.

## Ponytail (scope)

Ponytail controls how much to build and change. Its rules are the source of truth at `/home/dom/.gemini/config/plugins/ponytail/AGENTS.md`. Ponytail is installed as the user plugin `ponytail@ponytail`, not in this project.

- Apply it on every coding task.
- End each reply with one or two lines: what you skipped or did not check, and any risk.

## Rust (style)

Follow the `rust-best-practices` skill (`.factory/skills/rust-best-practices/SKILL.md`) for all Rust code: writing, reviewing, or refactoring. It is mandatory. Its source and update steps are in the skill's first section. Where its rules conflict with the Ponytail scope rules, follow *When the two rules disagree* below.

Before handing back Rust code, run and fix:

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Test scripts (required)

Agents must use these three scripts instead of typing the commands by hand. Run them from the repository root.

- `scripts/dev-test.sh [base]`: the fast loop while coding. It runs shellcheck on `scripts/*.sh`, fmt, clippy, and only the tests that cover the files changed against `base` (default `HEAD`, so your uncommitted work), then checks the test-count floor. Run it after each edit.
- `scripts/pre-pr-test.sh [base]`: run it before you push, and before you create or update a pull request. It needs a clean tree. It runs shellcheck, the gate self-test (`scripts/selftest-gates.sh`, which checks the mutant verdicts and the time limit against known inputs in a scratch copy), fmt, clippy, every test, the test-count floor, the doc build, `cargo deny` when dependencies changed, the price-key check when price or multiplier data changed, and mutants on the lines that differ from `base` (default `origin/main`). Its time limit is 90 seconds plus 2.5 seconds per mutant on the changed lines, at most 300 seconds, and it fails if it runs past the limit. If it times out, make the change smaller or run `full-test.sh`. It also fails when mutants are found on the changed lines but none of them compiles, because then the tests judged nothing.
- `scripts/full-test.sh`: the full regression, with no time limit and no step skipped. Run it before you merge to `main`, and before the final reply for any change under `src/` or `tests/`. It runs every branch of the Testing section below: shellcheck, the gate self-test, fmt, clippy, every test, the test-count floor, docs, `cargo deny`, the price-key check, Miri, mutants on the whole crate, both fuzz targets for 60 seconds each, and the 92% line-coverage floor. It runs every step even after a failure, then lists what failed.

The test-count floor is `MIN_TEST_COUNT` in `scripts/common.sh`. Raise it when you add tests. Lowering it needs the user's approval. Known equivalent mutants are listed in `scripts/known-equivalent-mutants.txt`. Add a line there only after you name that mutant as equivalent, with the reason, in your reply.

## Testing

Tight loop while coding: `scripts/dev-test.sh` (see Test scripts above). Keep it green after every change.

Before handing back, run the gate above, then every branch below that matches your change. Finish each branch before the reply. `scripts/full-test.sh` runs the gate and every branch in one command.

- **Library logic changed** (`src/*.rs` except `main.rs`): add or update a unit test in the same file. A bug fix starts with a regression test that fails before the fix.
- **Parser changed** (`droid`, `opencode`, `agy`, `prices`, `multipliers`): keep or add a proptest that the parser never panics on arbitrary input. `multipliers` and `prices` parse downloaded text, so after a change to either, also fuzz it: `mkdir -p fuzz/corpus/<target> && cargo +nightly fuzz run <target> fuzz/corpus/<target> fuzz/seeds/<target> -- -max_total_time=60 -dict=fuzz/dictionaries/<target>.dict`, with `<target>` as `multipliers_page` or `prices_table`. A crash is a bug: add a regression test that fails on the crash input, fix the code, and move the input into `fuzz/seeds/<target>/`.
- **Report layout changed** (new column, width, alignment, colors): extend the `report` tests. They check alignment, widths, and stripes, not only text.
- **Report output changed on purpose** (the golden test in `tests/golden.rs` fails): read the diff, then regenerate `tests/golden/report.txt` and `tests/golden/warnings.txt` with the command in `tests/golden/anonymize.py`. Changing those expected files is a test change, so it needs the user's approval first.
- **Price or multiplier data changed** (`LIST_KEYS`, multiplier rows): confirm each key exists in the newest snapshot under `~/.local/share/droid-usage/prices/`.
- **Pure logic changed**: run Miri on the nightly toolchain: `MIRIFLAGS=-Zmiri-disable-isolation PROPTEST_CASES=8 cargo +nightly miri test --lib -- --skip opencode --skip agy --skip app:: --skip download::` (about 2 minutes). Done means no undefined behavior is reported. `-Zmiri-disable-isolation` lets the file-based tests run (`scratch_dir()`, save and load). Without it Miri stops at `statx`. The skips drop the tests Miri cannot run: `opencode` and `agy` by name (SQLite), `app::` because the pipeline reads the OpenCode database, and `download::` because its loopback server needs sockets. While editing, add one module filter before the skips, for example `-- model:: --skip opencode --skip agy --skip app:: --skip download::` (about 3 seconds).
- **Logic changed in a module**: run mutants once for all touched modules: `cargo mutants --file src/<a>.rs --file src/<b>.rs --timeout 180`. `.cargo/mutants.toml` caps lints, so a mutant that only trips a warning still builds and gets judged. Each missed mutant either gets a new test or is named in the reply as equivalent, with the reason. Known equivalent: `agy.rs:79` and `opencode.rs:20` (`|` vs `^`), listed in `scripts/known-equivalent-mutants.txt`. Do not pass `--jobs 4`: it ran slower than the default.
- **New logic before final handback**: run `cargo llvm-cov --summary-only`. Line coverage stays at or above 92% as a floor. Mutation results judge test quality, not coverage.
- **Major change with a visual part** (report layout, colors, warnings, CLI output or flags): re-record `docs/demo.gif` with the `record-demo` skill (`.factory/skills/record-demo/SKILL.md`), check its preview frames, and commit the GIF with the change.
- **Dependencies changed** (`Cargo.toml`): run `cargo deny check`. Licenses and advisories pass.

Testing rules:

- Fixtures are inline strings, `scratch_dir()` folders, or the golden fixtures in `tests/golden/home`. Tests never read `HOME`, `~/.factory`, or the internet. The one network exception is the loopback server in `src/download.rs` tests: it binds `127.0.0.1` and answers only the test's own requests.
- Assertions check the value that matters, not only that a call succeeded.
- Do not add `#[ignore]` or weaken an assertion to get green. Report the failure instead.
- The full mutant run (`cargo mutants --timeout 180`, about 5 minutes over the whole crate) runs inside `scripts/full-test.sh`. `scripts/pre-pr-test.sh` mutates only the changed lines. Report every miss. If the full run keeps growing, split it with `--shard k/n`.
- Before editing, record the test count: `cargo test -q -- --list 2>/dev/null | grep -c ': test$'`. Before handback, the count is equal or higher. Fix the code under test when a test fails.
- A test is removed or changed only with the user's approval. If you think one is obsolete, ask, and name the test and the reason in the reply.

Known gaps, so do not assume they are covered:

- `main.rs` is the entry point only: it reads the environment and the clock. The run itself is `app::run`, tested with a fake download. `tests/binary.rs` runs the built binary offline. Put new behavior in `app.rs` or a library module, not `main.rs`.
- The live network is not tested. `download::fetch_both` is tested against a loopback server (error statuses, a cut-short body, a refused connection, a timeout), but no test fetches the real price table or models page. `refresh::refresh_prices` and `refresh::apply` are tested.
- The golden fixtures are anonymized copies of real sessions from one machine. They keep counts, model names, and timestamps, so they show real shapes, not every shape a machine can produce.
- No CI or pre-commit hooks.
- Fuzz targets cover only `multipliers` and `prices`. `droid`, `opencode`, and `agy` read local files and have proptests only.

## Swiss cheese layers

Each check is a slice of cheese with its own holes. The layers catch different faults, so their holes do not line up. No single layer proves the code correct. A green gate means every layer ran and found nothing, not that the code is right.

Layers, from cheapest to slowest:

- **Static**: `rustc`, fmt, and clippy with warnings denied catch type, style, and lint faults before any test runs.
- **Unit and integration tests** check values in each module and across modules.
- **Proptests** check that parsers never panic on arbitrary input.
- **Report tests** check layout, alignment, and stripes, which text assertions miss.
- **Data check** confirms price and multiplier keys exist in the newest snapshot.
- **Miri** checks pure logic for undefined behavior. It cannot run SQLite code, so those paths rely on other layers.
- **Fuzzing** (`cargo-fuzz`, in `fuzz/`) feeds coverage-guided random input to the two parsers of downloaded text and asserts their output invariants. It finds inputs that proptest does not generate, such as a zero promotion rate. It is slow, so it runs on parser changes, not every edit.
- **Mutation testing** (`cargo-mutants`) checks that tests fail when logic changes. A surviving mutant is a hole: add a test, or name it as equivalent in the reply.
- **Coverage** shows code that no test runs. It does not show that a test checks anything, so read it together with mutants.
- **Dependency audit** (`cargo-deny`) checks licenses and advisories.

Working with holes:

- A bug that escapes gets a regression test at the lowest layer that reproduces it. Then confirm the other layers still pass.
- Layers share blind spots when one author writes them all. Vary the inputs: generated inputs in proptests, hand-written fixtures in unit tests. When a bug escapes, ask which layers shared its blind spot.
- Pick layers with the branches above. When a change lands where no layer reaches (for example `main.rs` or the network path), say so in the reply. Do not claim it is covered.
- Coverage and mutation numbers guide the work. Write tests that catch real faults, not tests that only raise a number.

## When the two rules disagree

Ponytail decides what to build and how far to change. The Rust skill decides how the code is written.

- **Library crates only.** Builders, mocks behind `test-util`, sealed traits, full rustdoc on public items, and "I/O must be mockable" apply only to library crates, and only when the task asks for them. Application crates stay simple and follow Ponytail.
- **Refactors.** Fix only the smells the user named, inside the module the user named. Name every other smell as skipped in the closing line, and do not sweep the codebase.
- **Newtypes and wrappers.** Add a newtype only when mixing up two values is a real bug risk in the task at hand.
- **Comments.** Comment only the why. Library crates keep `///` docs on public items, with `# Errors` and `# Panics`. Application code gets no doc blocks unless asked.
- **Tests.** Non-trivial logic gets at least one small test (Ponytail's bar). Library public APIs follow the Rust skill's test rules.
- **Validation at trust boundaries and error handling that prevents data loss** always stay, whatever the scope rule says.
