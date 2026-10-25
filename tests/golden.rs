//! Golden test: the report for anonymized copies of real session files.
//!
//! `tests/golden/home` holds the fixtures, built by `tests/golden/anonymize.py`. They keep token counts, model names, timestamps, and costs, and they replace paths, IDs, and text. The expected output is `report.txt` and `warnings.txt` beside them. The file header of `anonymize.py` says how to regenerate both after a change.

use std::fs;
use std::path::Path;

use chrono::{TimeZone, Utc};
use droid_usage::app::{self, Mode};
use droid_usage::report::Colors;

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

    let want_report = fs::read_to_string(golden.join("report.txt")).unwrap();
    let want_warnings = fs::read_to_string(golden.join("warnings.txt")).unwrap();
    assert_eq!(
        String::from_utf8(out).unwrap(),
        want_report,
        "the report output matches the saved golden report file"
    );
    assert_eq!(
        String::from_utf8(err).unwrap(),
        want_warnings,
        "the warnings output matches the saved golden warnings file"
    );
}
