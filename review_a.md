# Comprehensive Codebase, Test Suite, and Test Harness Review (`review_a.md`)

> **Status**: Proposal awaiting user approval. **No source, test, or configuration files have been modified.**

---

## 1. Executive Summary & Baseline Metrics

The `droid-usage` crate is compact, well-structured, and heavily guarded by static and dynamic checks:

| Metric | Current Baseline |
| --- | --- |
| Edition / Rustc | Rust 2024 (`#![forbid(unsafe_code)]`, `warnings = "deny"`, `clippy::all` + `pedantic` + `nursery` + `cargo` + selected restriction lints) |
| Production modules (`src/`) | 13 files (`lib.rs`, `main.rs`, `app.rs`, `cli.rs`, `model.rs`, `prices.rs`, `multipliers.rs`, `refresh.rs`, `download.rs`, `droid.rs`, `opencode.rs`, `agy.rs`, `report.rs`) |
| Integration & golden tests (`tests/`) | 3 test binaries (`tests/binary.rs`, `tests/golden.rs`, `tests/readers.rs`) + fixture generator (`tests/golden/anonymize.py`) + recorded upstream samples (`tests/recorded/`) |
| Fuzz targets (`fuzz/`) | 2 `libfuzzer` targets (`prices_table`, `multipliers_page`) with dictionaries and seed corpora |
| Total `cargo test` count | **95** (`85` unit/proptests in `src/`, `2` in `tests/binary.rs`, `1` in `tests/golden.rs`, `7` in `tests/readers.rs`) |
| Total `cargo mutants --list` count | **259** mutants across `src/` |

---

## 2. Invariants That Must Be Preserved

Every proposed improvement below is designed to **strictly preserve (or strengthen)** all runtime, domain, and `AGENTS.md` invariants:

### 2.1 Domain & Runtime Invariants
1. **Price & Multiplier Validity**:
   - Every rate in a `prices::Snapshot` has finite, non-negative prices (`price.is_finite() && price >= 0.0`).
   - Missing `cache_read_input_token_cost` or `cache_creation_input_token_cost` in LiteLLM falls back to `input_cost_per_token` (overstates rather than understates cost); explicit `0.0` is preserved.
   - Every rate and promotion post-rate in a `multipliers::Snapshot` is finite and strictly positive (`rate.is_finite() && rate > 0.0`).
   - Promotion notes must match a parsed model row in both display name (after stripping `<sup>...`) and `total_cmp` rate; any malformed or unmatched note rejects the entire models page.
2. **Atomic Refresh Ordering (`refresh::apply`)**:
   - Both the price table and the models page must parse before either is written to disk.
   - `last-checked` is written last, so any parse or I/O failure leaves `last-checked` untouched and retries on the next run.
   - A missing multiplier snapshot forces a refresh even when `last-checked` is fresh (< 24h).
3. **Historical Pricing Semantics**:
   - A session is priced using the latest snapshot with `fetched <= session.started`, or the earliest saved snapshot when `session.started` precedes all snapshots.
   - Non-Droid sessions (`OpenCode`, `Agy`) keep a recorded `cost_usd` when present; otherwise they use `PriceBook::rate_at` (exact key match, then first `provider/<name>` match in `BTreeMap` key order, excluding multi-slash regional keys).
   - Droid sessions are priced exclusively by `multipliers::price_droid_sessions` using first-party `LIST_KEYS` (after stripping `-ultrafast` or `-fast`) and `PriceBook::exact_rate_at` (never provider fallback) multiplied by `Multiplier::on(session.started.date_naive())` (promotions end on a UTC `NaiveDate`).
4. **Resilience & Warning Semantics**:
   - Missing snapshot directories yield empty books (`Ok`), whereas unreadable existing directories return `Err`.
   - Corrupt `.json` files in snapshot directories are skipped and their filenames are sorted and reported on `stderr`; non-`.json` files are ignored silently.
   - Unreadable source entries increment `Read::skipped` and emit a warning; a missing `agy` transcript (`ErrorKind::NotFound`) is silently ignored (not counted as skipped).
   - `ErrorKind::BrokenPipe` when writing the report to `stdout` exits cleanly with `Ok(())`.
5. **Report Layout & Accounting**:
   - Weeks start on Monday (UTC) and include zero-total rows for empty weeks between the first and last active weeks.
   - `Totals::priced_tokens` sums tokens only for priced sessions so `cost per 1M tokens` has the same session denominator as `cost_usd`.
   - `Totals::unpriced` counts sessions where `tokens.total() > 0 && cost_usd.is_none()`.
   - Project names are shortened to `PROJECT_WIDTH` (40 terminal columns) with middle `…`, respecting Unicode character display widths.
   - Golden outputs (`tests/golden/report.txt` and `tests/golden/warnings.txt`) remain byte-for-byte identical.

### 2.2 `AGENTS.md` Engineering & Testing Gate Invariants
- `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test` pass with zero warnings.
- Total test count stays `>= 95`; no test is removed or altered without explicit user approval.
- Miri (`MIRIFLAGS=-Zmiri-disable-isolation PROPTEST_CASES=8 cargo +nightly miri test --lib -- --skip opencode --skip agy --skip app:: --skip download::`) passes with zero UB.
- `cargo mutants` on all touched modules has zero surviving non-equivalent mutants.
- `cargo llvm-cov --summary-only` line coverage remains `>= 92%`.
- `cargo +nightly fuzz` targets pass for any parser changes.

---

## 3. Production Code Findings & Consolidation Proposals (`src/`)

### Finding P1 (High — Trust-Boundary Invariant Gap): Snapshot JSON Deserialization Drops Corrupt Rate Entries Silently and Skips Numeric Validation
- **Locations**: `src/prices.rs:161-177` (`prices::Snapshot::from_json`) and `src/multipliers.rs:238-260` (`multipliers::Snapshot::from_json`).
- **Problem**:
  1. In `prices::Snapshot::from_json` (`src/prices.rs:166`), `.filter_map(...)` is used when deserializing the `"rates"` object:
     ```rust
     let rates = value["rates"]
         .as_object()?
         .iter()
         .filter_map(|(key, rate)| {
             let rate = Rate {
                 input: rate["input"].as_f64()?,
                 output: rate["output"].as_f64()?,
                 cache_read: rate["cache_read"].as_f64()?,
                 cache_write: rate["cache_write"].as_f64()?,
             };
             Some((key.clone(), rate))
         })
         .collect();
     ```
     If a saved price snapshot file has a corrupted model entry (e.g. `"openai/gpt-5.5": {"input": "bad"}` or truncated fields), `filter_map` **silently drops that model** and still returns `Some(Snapshot)` (even an empty `rates` map if all entries are corrupt!), rather than returning `None` so `read_snapshots` records the filename in `Loaded::skipped` and warns the user. By contrast, `multipliers::Snapshot::from_json` (`src/multipliers.rs:258`) uses `.map(...).collect::<Option<BTreeMap<_, _>>>()?`, which rejects the snapshot if any entry is malformed.
  2. Neither `prices::Snapshot::from_json` nor `multipliers::Snapshot::from_json` checks that deserialized numbers satisfy the domain invariants enforced by `parse_table` (`price.is_finite() && price >= 0.0`, non-empty `rates`) and `parse_page` (`rate.is_finite() && rate > 0.0`, `promo.after.is_finite() && promo.after > 0.0`, non-empty `models`).
- **Proposed Change**:
  - Extract `Rate::is_valid(&self) -> bool` in `src/prices.rs` (used in both `parse_table` and `Snapshot::from_json`) and use `.map(...).collect::<Option<BTreeMap<_, _>>>()?` + `(!rates.is_empty()).then_some(...)` in `prices::Snapshot::from_json`.
  - Extract `is_valid_multiplier(rate: f64) -> bool` in `src/multipliers.rs` (used in `multiplier_text` and `Snapshot::from_json`) and require `!models.is_empty()` in `multipliers::Snapshot::from_json`.
  - **Code sample (`src/prices.rs`)**:
    ```rust
    impl Rate {
        fn is_valid(&self) -> bool {
            [self.input, self.output, self.cache_read, self.cache_write]
                .into_iter()
                .all(|price| price.is_finite() && price >= 0.0)
        }
    }

    // In Snapshot::from_json:
    fn from_json(value: &Value) -> Option<Self> {
        let fetched = parse_timestamp(value["fetched"].as_str()?)?;
        let rates: BTreeMap<String, Rate> = value["rates"]
            .as_object()?
            .iter()
            .map(|(key, rate)| {
                let rate = Rate {
                    input: rate["input"].as_f64()?,
                    output: rate["output"].as_f64()?,
                    cache_read: rate["cache_read"].as_f64()?,
                    cache_write: rate["cache_write"].as_f64()?,
                };
                rate.is_valid().then(|| (key.clone(), rate))
            })
            .collect::<Option<BTreeMap<_, _>>>()?;
        (!rates.is_empty()).then_some(Self { fetched, rates })
    }
    ```
  - **Code sample (`src/multipliers.rs`)**:
    ```rust
    fn valid_multiplier(rate: f64) -> Option<f64> {
        (rate.is_finite() && rate > 0.0).then_some(rate)
    }

    // In Snapshot::from_json:
    fn from_json(value: &Value) -> Option<Self> {
        let fetched = parse_timestamp(value["fetched"].as_str()?)?;
        let models = value["models"]
            .as_object()?
            .iter()
            .map(|(id, entry)| {
                let promo = match &entry["promo"] {
                    Value::Null => None,
                    promo => Some(Promo {
                        through: NaiveDate::parse_from_str(promo["through"].as_str()?, "%Y-%m-%d")
                            .ok()?,
                        after: valid_multiplier(promo["after"].as_f64()?)?,
                    }),
                };
                let multiplier = Multiplier {
                    rate: valid_multiplier(entry["rate"].as_f64()?)?,
                    promo,
                };
                Some((id.clone(), multiplier))
            })
            .collect::<Option<BTreeMap<_, _>>>()?;
        (!models.is_empty()).then_some(Self { fetched, models })
    }
    ```

---

### Finding P2 (Medium — Duplication): Consolidate Snapshot Book Lookup & Persistence Between `prices` and `multipliers`
- **Locations**:
  - `src/prices.rs:182-223` (`PriceBook`) vs `src/multipliers.rs:265-301` (`MultiplierBook`)
  - `src/prices.rs:277-292, 359-367` (`read`, `load`, `save`) vs `src/multipliers.rs:346-376` (`read`, `load`, `save`)
- **Problem**:
  - `PriceBook` and `MultiplierBook` implement the exact same time-ordered snapshot lookup (`sort_by_key(|s| s.fetched)`, `is_empty`, `iter().rev().find(|s| s.fetched <= at).or_else(|| first())`, `snapshots.last()`).
  - `prices::save` and `multipliers::save` implement the exact same persistence logic: load existing book, compare newest map against the candidate snapshot's map, return `Refresh::Unchanged` if equal, else `fs::create_dir_all(dir)`, format `%Y%m%dT%H%M%SZ.json`, write `to_json().to_string()`, and return `Refresh::Updated`.
  - Because this logic is written twice, `cargo-mutants` also generates duplicate mutants for both modules (`is_empty`, `<=` -> `>` in `snapshot_in_effect` / `multiplier_at`, `newest_rates` / `newest`, `read`, `load`, `save`).
- **Proposed Consolidation**:
  - Keep the public types (`PriceBook`, `MultiplierBook`, `prices::Snapshot`, `multipliers::Snapshot`) and public functions (`prices::{read, load, save}`, `multipliers::{read, load, save}`) unchanged so no external caller or test signature breaks.
  - Add `pub(crate)` helpers in `src/prices.rs` (alongside the existing `pub(crate) fn read_snapshots`):
    1. `pub(crate) fn snapshot_in_effect<S>(snapshots: &[S], at: DateTime<Utc>, fetched: impl Fn(&S) -> DateTime<Utc>) -> Option<&S>`
    2. `pub(crate) fn save_snapshot(dir: &Path, fetched: DateTime<Utc>, json: &Value, unchanged: bool) -> io::Result<Refresh>`
  - **Code sample (`src/prices.rs`)**:
    ```rust
    pub(crate) fn snapshot_at<S>(
        snapshots: &[S],
        at: DateTime<Utc>,
        fetched: impl Fn(&S) -> DateTime<Utc>,
    ) -> Option<&S> {
        snapshots
            .iter()
            .rev()
            .find(|snapshot| fetched(snapshot) <= at)
            .or_else(|| snapshots.first())
    }

    pub(crate) fn write_snapshot(
        dir: &Path,
        fetched: DateTime<Utc>,
        json: &Value,
        unchanged: bool,
    ) -> io::Result<Refresh> {
        if unchanged {
            return Ok(Refresh::Unchanged);
        }
        fs::create_dir_all(dir)?;
        let name = format!("{}.json", fetched.format("%Y%m%dT%H%M%SZ"));
        fs::write(dir.join(name), json.to_string())?;
        Ok(Refresh::Updated)
    }
    ```
    Then `prices::save` becomes:
    ```rust
    pub fn save(dir: &Path, snapshot: &Snapshot) -> io::Result<Refresh> {
        let unchanged = load(dir)?.newest_rates() == Some(&snapshot.rates);
        write_snapshot(dir, snapshot.fetched, &snapshot.to_json(), unchanged)
    }
    ```
    And `multipliers::save` becomes:
    ```rust
    pub fn save(dir: &Path, snapshot: &Snapshot) -> io::Result<Refresh> {
        let unchanged = load(dir)?.newest() == Some(&snapshot.models);
        write_snapshot(dir, snapshot.fetched, &snapshot.to_json(), unchanged)
    }
    ```

---

### Finding P3 (Low — Intra-Module Duplication): `multipliers::model_row` Re-implements `multiplier_text`
- **Locations**: `src/multipliers.rs:176-177` and `src/multipliers.rs:186-189`.
- **Problem**:
  `model_row` manually parses and validates the `×`-suffixed multiplier instead of calling `multiplier_text` defined 9 lines below:
  ```rust
  // Currently in model_row (lines 176-177):
  let rate: f64 = cells.next()?.strip_suffix('×')?.parse().ok()?;
  (rate.is_finite() && rate > 0.0).then_some((plain_name(name), id, rate))

  // multiplier_text (lines 186-189):
  fn multiplier_text(text: &str) -> Option<f64> {
      let rate: f64 = text.strip_suffix('×')?.parse().ok()?;
      (rate.is_finite() && rate > 0.0).then_some(rate)
  }
  ```
  This duplication also produces 4 duplicate mutants in `cargo-mutants` (`&&` -> `||`, `>` -> `==`, `<`, `>=` at line 177).
- **Proposed Change**:
  ```rust
  fn model_row(line: &str) -> Option<(&str, &str, f64)> {
      if !line.starts_with('|') {
          return None;
      }
      let mut cells = line.split('|').map(str::trim).skip(1);
      let name = cells.next()?;
      let id = cells.next()?.strip_prefix('`')?.strip_suffix('`')?;
      let rate = multiplier_text(cells.next()?)?;
      Some((plain_name(name), id, rate))
  }
  ```

---

### Finding P4 (Low — Redundant Pass & Edge-Case Guard in `src/report.rs`): Compute `all_totals` Once and Guard `shorten` When `max == 0`
- **Locations**: `src/report.rs:236-265` (`shorten`) and `src/report.rs:351, 412` (`write_report`).
- **Problem**:
  1. `write_report` calls `all_totals(sessions)` twice: once at line 351 for the Weekly table's `total` row, and again at line 412 to check `if all.unpriced > 0`.
  2. In `shorten(text: &str, max: usize)` (`src/report.rs:240`), `let kept = max - 1;` will panic on integer underflow if `max == 0` and `text.width() > 0`. While production currently passes `PROJECT_WIDTH = 40`, making `shorten` total for all `max: usize` (e.g., `let Some(kept) = max.checked_sub(1) else { return String::new(); };`) prevents any underflow panic and allows property testing across all `max`.
- **Proposed Change**:
  - In `write_report`: compute `let all = all_totals(sessions);` once before the Weekly section and reuse `&all` at line 351 and `all.unpriced` at line 413.
  - In `shorten`: use `let Some(kept) = max.checked_sub(1) else { return String::new(); };`.

---

### Finding P5 (Low — Trust-Boundary Validation in `src/opencode.rs`): Sanitize Non-Finite or Negative `cost` Values from SQLite
- **Locations**: `src/opencode.rs:46, 60`.
- **Problem**:
  `opencode::read_from` guards against negative token counts (`count_column` converts negative integers to `0`), and `prices::parse_table` rejects negative or non-finite prices, but `opencode::read_from` accepts `row.get::<_, Option<f64>>(7)?` (`cost`) verbatim. If an OpenCode SQLite row has a negative or `NaN`/`Inf` float in `cost`, `session.cost_usd` becomes `Some(-1.0)` or `Some(NaN)`, which bypasses `prices::price_sessions` and corrupts `Totals::cost_usd` in the report.
- **Proposed Change**:
  Filter `cost` with `.filter(|c| c.is_finite() && *c >= 0.0)` in `opencode::read_from`:
  ```rust
  cost_usd: cost.filter(|c| c.is_finite() && *c >= 0.0),
  ```

---

### Finding P6 (Low — API Ergonomics in `src/cli.rs`): Generalize `parse_args` Input Bound
- **Locations**: `src/cli.rs:8` and `src/cli.rs:27-29`.
- **Problem**:
  `pub fn parse_args(args: impl Iterator<Item = String>) -> Result<bool, String>` requires owned `String` items, forcing `cli::tests` to define a `parse_strs` wrapper that allocates `String`s from `&[&str]`.
- **Proposed Change**:
  ```rust
  pub fn parse_args<I, S>(args: I) -> Result<bool, String>
  where
      I: IntoIterator<Item = S>,
      S: AsRef<str>,
  {
      let mut offline = false;
      for arg in args {
          match arg.as_ref() {
              "--offline" => offline = true,
              other => {
                  return Err(format!(
                      "unknown argument {other}; the only option is --offline"
                  ));
              }
          }
      }
      Ok(offline)
  }
  ```
  `main.rs` continues to call `cli::parse_args(env::args().skip(1))?` unchanged, while callers and tests can pass `&["--offline"]` directly.

---

## 4. Test Suite & Test Harness Findings & Consolidation Proposals (`tests/`, `src/*/tests`, `fuzz/`, `evals/`)

### Finding T1 (High — Test Hygiene & Duplication): 4 Duplicated Scratch-Directory Helpers Without RAII Cleanup
- **Locations**:
  1. `src/prices.rs:384-388`: `pub(crate) fn scratch_dir(label: &str) -> PathBuf` (removes dir, does not create it, no cleanup on drop).
  2. `src/model.rs:224`: `env::temp_dir().join(format!("droid-usage-test-{}", std::process::id()))` (ad-hoc temp path, manual `remove_dir_all`).
  3. `tests/readers.rs:20-25`: `fn scratch(label: &str) -> PathBuf` (removes and creates dir, manual `remove_dir_all`).
  4. `tests/binary.rs:15-20`: `fn scratch(label: &str) -> PathBuf` (exact copy of `tests/readers.rs:20-25`).
- **Problems**:
  - **Leaked `/tmp` directories**: Every test must remember to call `fs::remove_dir_all(&dir)` at the end. In fact, two tests in `src/refresh.rs` (`a_bad_page_saves_neither_snapshot` at lines 93-105 and `a_bad_table_saves_neither_snapshot` at lines 108-116) never call `fs::remove_dir_all(&root)`, and any test that panics mid-execution leaves its `/tmp/droid-usage-*` directory behind.
  - **Cross-process collision hazard during `cargo mutants`**: `scratch_dir(label)` in `src/prices.rs`, `tests/readers.rs`, and `tests/binary.rs` omits `std::process::id()`, so concurrent test processes (such as `cargo mutants` or parallel IDE + CLI runs) share the exact same `/tmp/droid-usage-{label}` path.
- **Proposed Consolidation**:
  - Replace `scratch_dir` with a lightweight RAII `ScratchDir` struct in `src/` (and a tiny shared helper or identical RAII wrapper in `tests/`) that:
    - Includes `std::process::id()` and `label` in the path: `std::env::temp_dir().join(format!("droid-usage-{}-{label}", std::process::id()))`.
    - Clears any prior directory on construction and removes it automatically in `impl Drop for ScratchDir`.
    - Implements `Deref<Target = Path>` and `AsRef<Path>` so `&dir` and `dir.join(...)` work identically to `PathBuf`.
  - **Code sample**:
    ```rust
    #[cfg(test)]
    pub(crate) struct ScratchDir(std::path::PathBuf);

    #[cfg(test)]
    impl ScratchDir {
        pub(crate) fn new(label: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("droid-usage-{}-{label}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            Self(dir)
        }
    }

    #[cfg(test)]
    impl std::ops::Deref for ScratchDir {
        type Target = Path;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    #[cfg(test)]
    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    ```

---

### Finding T2 (Medium — Duplicated Test Fixtures Across `src/`): Consolidate Shared Test Constants
- **Locations**:
  - `TABLE` and `PAGE` fixtures duplicated in `src/app.rs:118-121` and `src/refresh.rs:61-64`.
  - `RECORDED_TABLE` (`include_str!("../tests/recorded/litellm-sample.json")`) duplicated in `src/prices.rs:615` and `src/multipliers.rs:581`.
  - `no_download()` duplicated in `src/app.rs:128`, `src/refresh.rs:118`, and inline in `tests/golden.rs:21`.
- **Proposed Consolidation**:
  - Export `pub(crate) const RECORDED_TABLE: &str` from `prices::tests` (or a `#[cfg(test)]` block in `prices.rs`), and share `SAMPLE_TABLE`, `SAMPLE_PAGE`, and `no_download` between `refresh::tests` and `app::tests`.

---

### Finding T3 (Medium — Missing Automated Enforcement of `AGENTS.md` Data Invariant): Test `LIST_KEYS` Against Recorded Snapshots
- **Locations**: `src/multipliers.rs:22-67` (`LIST_KEYS`), `tests/recorded/litellm-sample.json`, `tests/recorded/models-page.md`.
- **Problem**:
  - `AGENTS.md` requires: *"Price or multiplier data changed (`LIST_KEYS`, multiplier rows): confirm each key exists in the newest snapshot under `~/.local/share/droid-usage/prices/`."*
  - Currently, `multipliers::tests` checks a single model (`gpt-5.6-sol`) against `RECORDED_TABLE` and `RECORDED_PAGE`, but does not check that **every** entry `(droid_id, litellm_key)` in `LIST_KEYS` actually resolves in `RECORDED_TABLE` and `RECORDED_PAGE`, or that `LIST_KEYS` has no duplicate keys.
  - *(Note: `tests/recorded/models-page.md` also lists `claude-fable-5.1` and several open models such as `inkling`, `qwen3.8-max`, `nemotron-*`, `minimax-*`, `muse-spark-1.2`, and `beam-1.0` that are intentionally omitted from `LIST_KEYS` because they have no first-party entry in `tests/recorded/litellm-sample.json`.)*
- **Proposed Improvement**:
  - Add a unit test `every_list_key_exists_in_the_recorded_table_and_models_page` in `src/multipliers.rs`:
    ```rust
    #[test]
    fn every_list_key_exists_in_the_recorded_table_and_models_page() {
        let prices = crate::prices::parse_table(RECORDED_TABLE, at(10, 25, 4)).unwrap();
        let page = parse_page(RECORDED_PAGE, at(10, 25, 4)).unwrap();
        for (index, (droid_id, litellm_key)) in LIST_KEYS.iter().enumerate() {
            assert!(
                LIST_KEYS[..index].iter().all(|(earlier, _)| earlier != droid_id),
                "duplicate LIST_KEYS id: {droid_id}"
            );
            assert!(
                page.models.contains_key(*droid_id),
                "LIST_KEYS model {droid_id} is missing from RECORDED_PAGE"
            );
            assert!(
                prices.rates.contains_key(*litellm_key),
                "LIST_KEYS target {litellm_key} for {droid_id} is missing from RECORDED_TABLE"
            );
        }
    }
    ```

---

### Finding T4 (Medium — Property-Based Testing Gaps): Add Proptests to `multipliers`, `opencode`, and `report::shorten`
- **Locations**: `src/multipliers.rs`, `src/opencode.rs`, `src/report.rs`.
- **Problem**:
  1. `AGENTS.md` states: *"Parser changed (`droid`, `opencode`, `agy`, `prices`, `multipliers`): keep or add a proptest that the parser never panics on arbitrary input."* Currently, `droid`, `agy`, and `prices` have `proptest!` blocks, but **`multipliers` and `opencode` have none**.
  2. In `src/report.rs`, `shorten(text, max)` slices strings and computes terminal display widths using `unicode_width`, which is a classic source of UTF-8 boundary and column-width bugs on combining marks, zero-width joiners, and 2-column CJK/emoji characters. It currently has two fixed unit tests and no `proptest!`.
- **Proposed Improvement**:
  1. In `src/multipliers.rs`: add `proptest!` verifying `parse_page(&text, at(10, 12, 0))` never panics on arbitrary text, and that `Snapshot::from_json(&snapshot.to_json())` round-trips for arbitrary generated `Multiplier` / `Promo` values.
  2. In `src/opencode.rs`: add `proptest!` verifying `model_id(&text)` and `started_at(&SqlValue::Text(text))` never panic on arbitrary strings.
  3. In `src/report.rs`: add `proptest!` verifying `shorten(&text, max)` never panics for arbitrary Unicode strings `text` and `max in 1usize..80`, and always satisfies `shorten(&text, max).width() <= max`.

---

### Finding T5 (Low — Fuzz Harness Enhancement): Assert Snapshot JSON Round-Trip in `fuzz/` Targets
- **Locations**: `fuzz/fuzz_targets/prices_table.rs:14-24` and `fuzz/fuzz_targets/multipliers_page.rs:14-30`.
- **Problem**:
  Both fuzz targets parse arbitrary UTF-8 input into a `Snapshot` and assert numeric invariants on the parsed struct, but neither exercises `Snapshot::to_json()` -> `Snapshot::from_json()` (which is the exact serialization path `save` and `read` use on every downloaded snapshot).
- **Proposed Improvement**:
  Make `Snapshot::from_json` public (or test via `to_json` round-trip if exposed) or keep `from_json` `pub(crate)` and test round-tripping in `proptest!`; if `from_json` is exposed as `#[must_use] pub fn from_json(value: &Value) -> Option<Self>` on `prices::Snapshot` and `multipliers::Snapshot` (matching the already-`pub` `Snapshot::to_json`), both fuzz targets can also assert `assert_eq!(Snapshot::from_json(&snapshot.to_json()).as_ref(), Some(&snapshot));`.

---

### Finding T6 (Low — Placement of Pure In-Memory Unit Tests in `tests/readers.rs`)
- **Locations**:
  - `tests/readers.rs:69-80` (`droid_header_keeps_the_first_project_seen`)
  - `tests/readers.rs:196-217` (`opencode_integer_timestamps_are_read_when_the_column_keeps_integers`)
- **Problem**:
  - `tests/readers.rs` is documented as `//! Reader tests against real folders and databases, through the public API.`
  - However, `droid_header_keeps_the_first_project_seen` calls `droid::parse_header(Cursor::new(text))` purely in-memory (which means `cargo +nightly miri test --lib` skips it because it lives in an integration test binary instead of `src/droid.rs`), and `opencode_integer_timestamps_are_read_when_the_column_keeps_integers` uses `Connection::open_in_memory()` + `opencode::read_from(&conn)` just like the unit tests in `src/opencode.rs`.
- **Proposed Options (Requires User Approval per `AGENTS.md`)**:
  - **Option 6A**: Move `droid_header_keeps_the_first_project_seen` into `src/droid.rs` (so Miri `--lib` executes it) and `opencode_integer_timestamps_are_read_when_the_column_keeps_integers` into `src/opencode.rs` (total test count stays identical).
  - **Option 6B**: Keep both tests in `tests/readers.rs` untouched if you prefer preserving the exact file-level test layout for `evals/` baseline comparisons.

---

### Finding T7 (Informational — Diagnosis of the `cli.rs` Mutation Finding in `evals/since-flag-ab-2026-10-24.md`)
- **Location**: `evals/since-flag-ab-2026-10-24.md:76, 96`.
- **Diagnosis**:
  - The A/B evaluation report noted: *"The `cli.rs` mutation run produced one mutant, so it checks almost nothing. The cause was not investigated."*
  - **Root cause**: `cargo-mutants` generates function-body return replacements (`Ok(Default::default())`, `Ok(true)`, `Ok(false)`) only when the `Ok` type is `bool`, a primitive, or implements `Default`, and generates interior mutants only on binary/unary operators (`==`, `!=`, `&&`, `!`, `<`, `+`, etc.) or match guards. Pattern-matching on string literals (`match arg.as_str() { "--offline" => ... }`) contains no operators. In the baseline `src/cli.rs`, `parse_args -> Result<bool, String>` generates 2 mutants (`Ok(true)` and `Ok(false)`). When the A/B worker changed the return type to `Result<Args, String>` *without* deriving `Default` on `Args` and inlined date parsing inside the `match` arm, `cargo-mutants` could not construct `Ok(Default::default())` and only generated 1 unviable mutant.
  - **Takeaway for future CLI changes**: Any `Args` struct returned from `cli::parse_args` should `#[derive(Default)]`, and value parsers (e.g. `parse_since`) should be small helper functions returning `Option<NaiveDate>` or `Result<NaiveDate, String>` so `cargo-mutants` generates return-value mutants for each helper.

---

## 5. Proposed Implementation Plan (Phased, Pending Approval)

If you approve all or a subset of the items above, we will execute them in small, verified steps:

1. **Record Baseline**:
   - Confirm test count (`95`) and baseline checks before any edit.
2. **Phase 1 — Trust-Boundary & Validation Tightening (P1, P4, P5)**:
   - Write failing unit tests first for:
     - Corrupt individual rate entry in `prices::Snapshot::from_json` and negative/non-finite numbers in saved price and multiplier snapshots (`P1`).
     - Negative / `NaN` / `Inf` `cost` values in `opencode::read_from` (`P5`).
     - `shorten("abc", 0)` edge case (`P4`).
   - Implement `P1`, `P4` (including single `all_totals` computation in `write_report`), and `P5`.
3. **Phase 2 — Codebase Consolidation & Idiom Cleanup (P2, P3, P6)**:
   - Consolidate `snapshot_at` and `write_snapshot` across `src/prices.rs` and `src/multipliers.rs` (`P2`).
   - Reuse `multiplier_text` in `multipliers::model_row` (`P3`).
   - Generalize `cli::parse_args` to accept `impl IntoIterator<Item = impl AsRef<str>>` (`P6`).
4. **Phase 3 — Test Suite & Harness Consolidation (T1, T2, T3, T4, T5, and optionally T6)**:
   - Introduce RAII `ScratchDir` guard and replace manual `scratch_dir` / `scratch` + `fs::remove_dir_all` calls (`T1`, `T2`).
   - Add `every_list_key_exists_in_the_recorded_table_and_models_page` unit test (`T3`).
   - Add `proptest!` blocks in `src/multipliers.rs`, `src/opencode.rs`, and `src/report.rs` (`T4`).
   - Expose `Snapshot::from_json` and assert round-tripping in both `fuzz/fuzz_targets/*.rs` (`T5`).
   - Move or keep the two in-memory tests in `tests/readers.rs` per your choice on `T6`.
5. **Phase 4 — Full Swiss-Cheese Verification Gate (`AGENTS.md`)**:
   - `cargo fmt --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test` (confirm test count `> 95` and golden tests pass untouched)
   - `MIRIFLAGS=-Zmiri-disable-isolation PROPTEST_CASES=8 cargo +nightly miri test --lib -- --skip opencode --skip agy --skip app:: --skip download::`
   - `cargo mutants` on all touched modules (`src/prices.rs`, `src/multipliers.rs`, `src/report.rs`, `src/opencode.rs`, `src/cli.rs`, `src/refresh.rs`, `src/app.rs`, `src/model.rs`, `src/droid.rs`)
   - `cargo +nightly fuzz run` (60s each for `prices_table` and `multipliers_page`)
   - `cargo llvm-cov --summary-only` (confirm line coverage `>= 92%`)

---

## 6. Approval Checklist

Please let me know which items you approve for implementation:

- [ ] **P1**: Enforce full validation and reject partial/corrupt entries in `prices::Snapshot::from_json` and `multipliers::Snapshot::from_json`
- [ ] **P2**: Consolidate snapshot lookup (`snapshot_at`) and persistence (`write_snapshot`) between `src/prices.rs` and `src/multipliers.rs`
- [ ] **P3**: Reuse `multiplier_text` in `multipliers::model_row`
- [ ] **P4**: Compute `all_totals(sessions)` once in `report::write_report` and guard `shorten` when `max == 0`
- [ ] **P5**: Filter negative/non-finite `cost` values in `opencode::read_from`
- [ ] **P6**: Generalize `cli::parse_args` to `impl IntoIterator<Item = impl AsRef<str>>`
- [ ] **T1 & T2**: Consolidate the 4 scratch-directory helpers into an RAII `ScratchDir` guard and deduplicate test fixtures
- [ ] **T3**: Add automated test verifying `LIST_KEYS` against `RECORDED_TABLE` and `RECORDED_PAGE`
- [ ] **T4**: Add `proptest!` coverage for `multipliers::parse_page`, `opencode::{model_id, started_at}`, and `report::shorten`
- [ ] **T5**: Assert `Snapshot::to_json` / `from_json` round-tripping in `fuzz/fuzz_targets/`
- [ ] **T6**: Move the two pure in-memory tests from `tests/readers.rs` into `src/droid.rs` and `src/opencode.rs` (Option 6A) vs. keep them in `tests/readers.rs` (Option 6B)
