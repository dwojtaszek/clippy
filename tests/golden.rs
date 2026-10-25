//! Golden test: the report for anonymized copies of real session files.
//!
//! `tests/golden/home` holds the fixtures, built by `tests/golden/anonymize.py`. They keep token counts, model names, timestamps, and costs, and they replace paths, IDs, and text. The expected output is `report.txt` and `warnings.txt` beside them. The file header of `anonymize.py` says how to regenerate both after a change.

use std::path::Path;

use chrono::{TimeZone, Utc};
use droid_usage::app::{self, Mode};
use droid_usage::report::Colors;
use snapbox::{Assert, Data};

#[test]
fn report_matches_the_golden_output_for_anonymized_sessions() {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let now = Utc.with_ymd_and_hms(2026, 10, 25, 12, 0, 0).unwrap();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    app::run(
        &golden.join("home"),
        Mode::Offline,
        now,
        Colors::Off,
        || Err("network should not be used".into()),
        &mut out,
        &mut err,
    )
    .unwrap();

    // Raw data has no wildcard or path filters, so the saved files must match byte for byte
    // (line endings aside). `Assert::new` ignores the SNAPSHOTS variable, so a shell setting cannot skip the check.
    let want_report = Data::read_from(&golden.join("report.txt"), None).raw();
    let want_warnings = Data::read_from(&golden.join("warnings.txt"), None).raw();
    Assert::new().eq(String::from_utf8(out).unwrap(), want_report);
    Assert::new().eq(String::from_utf8(err).unwrap(), want_warnings);
}
