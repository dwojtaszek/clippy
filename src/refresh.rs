//! Saves a fresh price table and Droid models page together, or neither.

use std::error::Error;
use std::path::Path;

use chrono::{DateTime, Utc};

use crate::download::Downloads;
use crate::{multipliers, prices};

/// Parses both downloads, then saves both snapshots.
///
/// Nothing is saved unless both parse, so prices and multipliers always come from the same check. The check time is written last, so a failed save is retried on the next run.
///
/// # Errors
///
/// Returns an error when either text is malformed or a snapshot cannot be written.
pub fn apply(
    prices_dir: &Path,
    multipliers_dir: &Path,
    now: DateTime<Utc>,
    table: &str,
    page: &str,
) -> Result<(), Box<dyn Error>> {
    let prices = prices::parse_table(table, now)?;
    let multipliers = multipliers::parse_page(page, now)?;
    let _ = prices::save(prices_dir, &prices)?;
    let _ = multipliers::save(multipliers_dir, &multipliers)?;
    prices::record_check(prices_dir, now)?;
    Ok(())
}

/// Refreshes the saved snapshots when the last check is more than a day old, or when no multiplier snapshot is saved. `download` returns the price table and the models page. Returns a warning when the download or the save fails, and the saved snapshots stay as they were.
pub fn refresh_prices(
    prices_dir: &Path,
    multipliers_dir: &Path,
    now: DateTime<Utc>,
    download: impl FnOnce() -> Result<Downloads, Box<dyn Error>>,
) -> Option<String> {
    // A missing multiplier snapshot forces a download, so Droid costs start on the first run after an upgrade.
    let have_multipliers = multipliers::load(multipliers_dir).is_ok_and(|book| !book.is_empty());
    if have_multipliers && !prices::is_stale(prices_dir, now) {
        return None;
    }
    download()
        .and_then(|downloads| {
            apply(
                prices_dir,
                multipliers_dir,
                now,
                &downloads.table,
                &downloads.page,
            )
        })
        .err()
        .map(|err| format!("price download failed, using saved snapshots: {err}"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use chrono::{TimeDelta, TimeZone};

    use super::*;
    use crate::prices::last_checked;
    use crate::testing::{SAMPLE_PAGE, SAMPLE_TABLE, ScratchDir, no_download};

    fn at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 12, 12, 0, 0).unwrap()
    }

    #[test]
    fn both_snapshots_are_saved_and_the_check_is_recorded() {
        let root = ScratchDir::new("refresh-ok");
        let (prices_dir, multipliers_dir) = (root.join("prices"), root.join("multipliers"));
        apply(
            &prices_dir,
            &multipliers_dir,
            at(),
            SAMPLE_TABLE,
            SAMPLE_PAGE,
        )
        .unwrap();

        assert!(
            prices::load(&prices_dir)
                .unwrap()
                .rate_at("gpt-5.6-sol", at())
                .is_some(),
            "the saved price table has a rate for the model at the check time"
        );
        assert_eq!(
            multipliers::load(&multipliers_dir)
                .unwrap()
                .multiplier_at("gpt-5.6-sol", at()),
            Some(1.6),
            "the saved multiplier table gives the promotional multiplier for the model"
        );
        assert_eq!(
            last_checked(&prices_dir),
            Some(at()),
            "the price snapshot records the time of the successful check"
        );
    }

    #[test]
    fn a_bad_page_saves_neither_snapshot() {
        let root = ScratchDir::new("refresh-bad-page");
        let (prices_dir, multipliers_dir) = (root.join("prices"), root.join("multipliers"));
        let bad_page = SAMPLE_PAGE.replace(
            "Promotional pricing. GPT-5.6 Sol",
            "Promotional pricing. Ghost",
        );
        assert!(
            apply(&prices_dir, &multipliers_dir, at(), SAMPLE_TABLE, &bad_page).is_err(),
            "a page missing the expected model name fails and saves nothing"
        );

        assert!(
            prices::load(&prices_dir).unwrap().is_empty(),
            "no price snapshot is written when the page is bad"
        );
        assert!(
            multipliers::load(&multipliers_dir).unwrap().is_empty(),
            "no multiplier snapshot is written when the page is bad"
        );
        assert_eq!(
            last_checked(&prices_dir),
            None,
            "the check time is not recorded when the page is bad"
        );
    }

    #[test]
    fn a_bad_table_saves_neither_snapshot() {
        let root = ScratchDir::new("refresh-bad-table");
        let (prices_dir, multipliers_dir) = (root.join("prices"), root.join("multipliers"));
        assert!(
            apply(&prices_dir, &multipliers_dir, at(), "not json", SAMPLE_PAGE).is_err(),
            "a table that is not valid json fails and saves nothing"
        );

        assert!(
            prices::load(&prices_dir).unwrap().is_empty(),
            "no price snapshot is written when the table is bad"
        );
        assert!(
            multipliers::load(&multipliers_dir).unwrap().is_empty(),
            "no multiplier snapshot is written when the table is bad"
        );
        assert_eq!(
            last_checked(&prices_dir),
            None,
            "the check time is not recorded when the table is bad"
        );
    }

    #[test]
    fn a_fresh_check_with_multipliers_does_not_download() {
        let root = ScratchDir::new("refresh-fresh");
        let (prices_dir, multipliers_dir) = (root.join("prices"), root.join("multipliers"));
        apply(
            &prices_dir,
            &multipliers_dir,
            at(),
            SAMPLE_TABLE,
            SAMPLE_PAGE,
        )
        .unwrap();

        let later = at() + TimeDelta::hours(23);
        assert_eq!(
            refresh_prices(&prices_dir, &multipliers_dir, later, no_download),
            None,
            "a check within a day with snapshots present does not download"
        );
        assert_eq!(
            last_checked(&prices_dir),
            Some(at()),
            "a fresh check leaves the recorded check time unchanged"
        );
    }

    #[test]
    fn a_check_older_than_a_day_downloads_and_records_the_new_check() {
        let root = ScratchDir::new("refresh-stale");
        let (prices_dir, multipliers_dir) = (root.join("prices"), root.join("multipliers"));
        apply(
            &prices_dir,
            &multipliers_dir,
            at(),
            SAMPLE_TABLE,
            SAMPLE_PAGE,
        )
        .unwrap();

        let later = at() + TimeDelta::hours(25);
        let download = || {
            Ok(Downloads {
                table: SAMPLE_TABLE.to_owned(),
                page: SAMPLE_PAGE.to_owned(),
            })
        };
        assert_eq!(
            refresh_prices(&prices_dir, &multipliers_dir, later, download),
            None,
            "a check older than a day triggers a download with no warning"
        );
        assert_eq!(
            last_checked(&prices_dir),
            Some(later),
            "the recorded check time moves to the time of the new download"
        );
    }

    #[test]
    fn a_missing_multiplier_snapshot_forces_a_download() {
        let root = ScratchDir::new("refresh-no-multipliers");
        let (prices_dir, multipliers_dir) = (root.join("prices"), root.join("multipliers"));
        apply(
            &prices_dir,
            &multipliers_dir,
            at(),
            SAMPLE_TABLE,
            SAMPLE_PAGE,
        )
        .unwrap();
        fs::remove_dir_all(&multipliers_dir).unwrap();

        let download = || {
            Ok(Downloads {
                table: SAMPLE_TABLE.to_owned(),
                page: SAMPLE_PAGE.to_owned(),
            })
        };
        assert_eq!(
            refresh_prices(&prices_dir, &multipliers_dir, at(), download),
            None,
            "a missing multiplier snapshot triggers a download with no warning"
        );
        assert_eq!(
            multipliers::load(&multipliers_dir)
                .unwrap()
                .multiplier_at("gpt-5.6-sol", at()),
            Some(1.6),
            "the re-downloaded multiplier snapshot gives the promotional multiplier"
        );
    }

    #[test]
    fn a_failed_download_keeps_the_saved_snapshots_and_warns() {
        let root = ScratchDir::new("refresh-failed");
        let (prices_dir, multipliers_dir) = (root.join("prices"), root.join("multipliers"));
        apply(
            &prices_dir,
            &multipliers_dir,
            at(),
            SAMPLE_TABLE,
            SAMPLE_PAGE,
        )
        .unwrap();

        let later = at() + TimeDelta::hours(25);
        let warning = refresh_prices(&prices_dir, &multipliers_dir, later, || {
            Err("connection refused".into())
        });
        assert_eq!(
            warning.as_deref(),
            Some("price download failed, using saved snapshots: connection refused"),
            "a failed download returns a warning naming the error and keeps old snapshots"
        );
        assert_eq!(
            last_checked(&prices_dir),
            Some(at()),
            "a failed download does not move the recorded check time"
        );
    }
}
