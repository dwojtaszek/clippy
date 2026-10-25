#![no_main]

use chrono::{TimeZone, Utc};
use droid_usage::multipliers::{Snapshot, parse_page};
use libfuzzer_sys::fuzz_target;

// The models page is downloaded text. Any input must either parse or return an error,
// and a parsed page must hold only finite, positive multipliers and must read back
// unchanged once saved.
fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let fetched = Utc.with_ymd_and_hms(2026, 10, 24, 0, 0, 0).unwrap();
    let Ok(snapshot) = parse_page(text, fetched) else {
        return;
    };
    for (id, multiplier) in &snapshot.models {
        assert!(
            multiplier.rate.is_finite() && multiplier.rate > 0.0,
            "{id}: bad multiplier {}",
            multiplier.rate
        );
        if let Some(promo) = multiplier.promo {
            assert!(
                promo.after.is_finite() && promo.after > 0.0,
                "{id}: bad promotion rate {}",
                promo.after
            );
        }
    }
    assert_eq!(Snapshot::from_json(&snapshot.to_json()), Some(snapshot));
});
