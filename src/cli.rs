//! Command-line arguments. The parser lives in the library so its tests can reach it.

use std::error::Error;
use std::fmt::{self, Display};

use crate::app::Mode;

/// An argument other than `--offline`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownArgument(String);

impl Display for UnknownArgument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown argument {}; the only option is --offline",
            self.0
        )
    }
}

impl Error for UnknownArgument {}

/// Reads the arguments: `--offline` selects [`Mode::Offline`], and no arguments select [`Mode::Online`]. Any other argument is an error.
///
/// # Errors
///
/// Returns an [`UnknownArgument`] that names the first argument that is not `--offline`.
pub fn parse_args<I, S>(args: I) -> Result<Mode, UnknownArgument>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut mode = Mode::Online;
    for arg in args {
        match arg.as_ref() {
            "--offline" => mode = Mode::Offline,
            other => return Err(UnknownArgument(other.to_owned())),
        }
    }
    Ok(mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_arguments_means_online() {
        assert_eq!(
            parse_args(Vec::<&str>::new()),
            Ok(Mode::Online),
            "no arguments select online mode"
        );
    }

    #[test]
    fn offline_flag_is_read() {
        assert_eq!(
            parse_args(["--offline"]),
            Ok(Mode::Offline),
            "the offline flag selects offline mode"
        );
    }

    #[test]
    fn unknown_argument_is_named() {
        let err = parse_args(["--offline", "--bogus"]).unwrap_err();
        assert!(err.to_string().contains("--bogus"), "{err}");
    }
}
