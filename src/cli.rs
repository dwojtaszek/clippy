//! Command-line arguments. The parser lives in the library so its tests can reach it.

/// Returns whether `--offline` was given. Any other argument is an error.
///
/// # Errors
///
/// Returns an error that names the first argument that is not `--offline`.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_arguments_means_online() {
        assert_eq!(parse_args(Vec::<&str>::new()), Ok(false));
    }

    #[test]
    fn offline_flag_is_read() {
        assert_eq!(parse_args(["--offline"]), Ok(true));
    }

    #[test]
    fn unknown_argument_is_named() {
        let err = parse_args(["--offline", "--bogus"]).unwrap_err();
        assert!(err.contains("--bogus"), "{err}");
    }
}
