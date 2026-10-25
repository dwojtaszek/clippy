//! The whole run: read the sources, refresh prices, price the sessions, and write the report.
//!
//! `main` supplies the real home folder, clock, terminal, and download. This module takes them as arguments, so tests can run the whole flow.

use std::error::Error;
use std::fmt::Display;
use std::io::{ErrorKind, Write};
use std::path::Path;

use chrono::{DateTime, Utc};

use crate::model::{self, Session};
use crate::{agy, droid, multipliers, opencode, prices, refresh, report};

/// Colors only for a terminal, and never when `NO_COLOR` is set.
#[must_use]
pub const fn use_color(terminal: bool, no_color_set: bool) -> bool {
    terminal && !no_color_set
}

/// Runs the report. The report goes to `out`. Warnings go to `err`.
///
/// # Errors
///
/// Returns an error when a saved snapshot cannot be read or the report cannot be written. An unreadable source is only a warning.
pub fn run(
    home: &Path,
    offline: bool,
    now: DateTime<Utc>,
    color: bool,
    download: impl FnOnce() -> Result<(String, String), String>,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let data_dir = home.join(".local/share/droid-usage");
    let prices_dir = data_dir.join("prices");
    let multipliers_dir = data_dir.join("multipliers");

    let mut sessions: Vec<Session> = Vec::new();
    let source_warnings = [
        model::add_source(
            "droid",
            droid::read_sessions(&home.join(".factory/sessions")),
            &mut sessions,
        ),
        model::add_source(
            "opencode",
            opencode::read_sessions(&home.join(".local/share/opencode/opencode.db")),
            &mut sessions,
        ),
        model::add_source(
            "agy",
            agy::read_sessions(&home.join(".gemini/antigravity-cli")),
            &mut sessions,
        ),
    ];
    for warning in source_warnings.into_iter().flatten() {
        warn(err, warning);
    }

    let refresh_warning = if offline {
        None
    } else {
        refresh::refresh_prices(&prices_dir, &multipliers_dir, now, download)
    };
    if let Some(warning) = refresh_warning {
        warn(err, warning);
    }

    let prices_read = prices::read(&prices_dir)?;
    for name in &prices_read.skipped {
        warn(err, format!("skipped unreadable price snapshot {name}"));
    }
    let book = prices_read.book;
    if book.is_empty() {
        warn(
            err,
            "no price snapshot saved; sessions without a recorded cost are left blank",
        );
    }
    let multipliers_read = multipliers::read(&multipliers_dir)?;
    for name in &multipliers_read.skipped {
        warn(
            err,
            format!("skipped unreadable multiplier snapshot {name}"),
        );
    }
    let multiplier_book = multipliers_read.book;
    if multiplier_book.is_empty() {
        warn(
            err,
            "no multiplier snapshot saved; Droid sessions are left blank",
        );
    }
    prices::price_sessions(&mut sessions, &book);
    multipliers::price_droid_sessions(&mut sessions, &book, &multiplier_book);

    // Exit quietly when the output is piped into something that stops reading, like `head`.
    match report::write_report(out, &sessions, color) {
        Err(error) if error.kind() != ErrorKind::BrokenPipe => Err(error.into()),
        _ => Ok(()),
    }
}

/// Writes a warning line. Failing to write one is not worth stopping for.
fn warn(err: &mut impl Write, message: impl Display) {
    let _ = writeln!(err, "{message}");
}

#[cfg(test)]
mod tests {
    use std::{fs, io};

    use super::*;
    use crate::refresh::apply;
    use crate::testing::{SAMPLE_PAGE, SAMPLE_TABLE, ScratchDir, no_download};

    fn at() -> DateTime<Utc> {
        use chrono::TimeZone;
        Utc.with_ymd_and_hms(2026, 10, 25, 12, 0, 0).unwrap()
    }

    /// A writer that fails every write with the given kind.
    struct Failing(ErrorKind);

    impl Write for Failing {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(self.0))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn color_needs_a_terminal_and_no_no_color() {
        assert!(use_color(true, false));
        assert!(!use_color(true, true));
        assert!(!use_color(false, false));
    }

    #[test]
    fn offline_run_with_nothing_saved_warns_and_still_reports() {
        let home = ScratchDir::new("app-empty");
        let (mut out, mut err) = (Vec::new(), Vec::new());
        run(&home, true, at(), false, no_download, &mut out, &mut err).unwrap();

        let out = String::from_utf8(out).unwrap();
        let err = String::from_utf8(err).unwrap();
        assert!(out.contains("By source"), "{out}");
        assert!(err.contains("no price snapshot saved"), "{err}");
        assert!(err.contains("no multiplier snapshot saved"), "{err}");
    }

    #[test]
    fn a_corrupt_snapshot_is_named_in_a_warning() {
        let home = ScratchDir::new("app-corrupt");
        let prices_dir = home.join(".local/share/droid-usage/prices");
        fs::create_dir_all(&prices_dir).unwrap();
        fs::write(prices_dir.join("20261001T000000Z.json"), "{truncated").unwrap();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        run(&home, true, at(), false, no_download, &mut out, &mut err).unwrap();

        let err = String::from_utf8(err).unwrap();
        assert!(
            err.contains("skipped unreadable price snapshot 20261001T000000Z.json"),
            "{err}"
        );
    }

    #[test]
    fn a_failed_download_warns_and_the_report_still_runs() {
        let home = ScratchDir::new("app-failed");
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let download = || Err("connection refused".to_owned());
        run(&home, false, at(), false, download, &mut out, &mut err).unwrap();

        let err = String::from_utf8(err).unwrap();
        assert!(
            err.contains("price download failed, using saved snapshots: connection refused"),
            "{err}"
        );
        let report = String::from_utf8(out).unwrap();
        assert!(report.contains("By source"), "{report}");
    }

    #[test]
    fn a_successful_download_is_saved_before_the_prices_are_read() {
        let home = ScratchDir::new("app-downloaded");
        let download = || Ok((SAMPLE_TABLE.to_owned(), SAMPLE_PAGE.to_owned()));
        let (mut out, mut err) = (Vec::new(), Vec::new());
        run(&home, false, at(), false, download, &mut out, &mut err).unwrap();

        let err = String::from_utf8(err).unwrap();
        assert!(!err.contains("no price snapshot saved"), "{err}");
        assert!(!err.contains("no multiplier snapshot saved"), "{err}");
        assert!(home.join(".local/share/droid-usage/prices").is_dir());
    }

    #[test]
    fn color_reaches_the_report() {
        let home = ScratchDir::new("app-color");
        let (mut colored, mut plain, mut err) = (Vec::new(), Vec::new(), Vec::new());
        run(&home, true, at(), true, no_download, &mut colored, &mut err).unwrap();
        run(&home, true, at(), false, no_download, &mut plain, &mut err).unwrap();

        assert!(
            colored.contains(&0x1b),
            "colored report has no escape codes"
        );
        assert!(!plain.contains(&0x1b), "plain report has escape codes");
    }

    #[test]
    fn a_closed_pipe_is_not_an_error() {
        let home = ScratchDir::new("app-pipe");
        let mut out = Failing(ErrorKind::BrokenPipe);
        let mut err = Vec::new();
        assert!(run(&home, true, at(), false, no_download, &mut out, &mut err).is_ok());
    }

    #[test]
    fn other_write_failures_are_errors() {
        let home = ScratchDir::new("app-write-fail");
        let mut out = Failing(ErrorKind::PermissionDenied);
        let mut err = Vec::new();
        assert!(run(&home, true, at(), false, no_download, &mut out, &mut err).is_err());
    }

    #[test]
    fn the_saved_check_time_is_not_touched_by_an_offline_run() {
        let home = ScratchDir::new("app-offline-keeps");
        let prices_dir = home.join(".local/share/droid-usage/prices");
        let multipliers_dir = home.join(".local/share/droid-usage/multipliers");
        apply(
            &prices_dir,
            &multipliers_dir,
            at(),
            SAMPLE_TABLE,
            SAMPLE_PAGE,
        )
        .unwrap();

        let (mut out, mut err) = (Vec::new(), Vec::new());
        let later = at() + chrono::TimeDelta::days(3);
        run(&home, true, later, false, no_download, &mut out, &mut err).unwrap();

        assert_eq!(prices::last_checked(&prices_dir), Some(at()));
    }
}
