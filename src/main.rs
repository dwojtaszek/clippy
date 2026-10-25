//! Command-line entry point. It supplies the real home folder, clock, terminal, and network to the library.

use std::env;
use std::error::Error;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use chrono::Utc;
use droid_usage::{app, cli, download, multipliers, prices};

/// How long each download may take before it is abandoned.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60);

fn main() -> Result<(), Box<dyn Error>> {
    let offline = cli::parse_args(env::args().skip(1))?;
    let home = PathBuf::from(env::var("HOME")?);
    let color = app::use_color(console::colors_enabled(), env::var_os("NO_COLOR").is_some());
    app::run(
        &home,
        offline,
        Utc::now(),
        color,
        || {
            download::fetch_both(
                prices::SOURCE_URL,
                multipliers::SOURCE_URL,
                DOWNLOAD_TIMEOUT,
            )
            .map_err(|err| err.to_string())
        },
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    )
}
