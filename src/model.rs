//! Types shared by the readers, the prices, and the report.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

/// Where a session was recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Source {
    /// Droid CLI sessions under `~/.factory/sessions`.
    Droid,
    /// `OpenCode` sessions in its `SQLite` database.
    OpenCode,
    /// agy (Antigravity CLI) conversations.
    Agy,
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Droid => "droid",
            Self::OpenCode => "opencode",
            Self::Agy => "agy",
        })
    }
}

/// Token counts for one session. Reasoning tokens are not tracked; each source reports them differently.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Tokens {
    /// Prompt tokens that were not served from cache.
    pub input: u64,
    /// Completion tokens.
    pub output: u64,
    /// Prompt tokens served from cache.
    pub cache_read: u64,
    /// Prompt tokens written to cache.
    pub cache_write: u64,
}

impl Tokens {
    /// All four counts added together.
    #[must_use]
    pub const fn total(&self) -> u64 {
        self.input + self.output + self.cache_read + self.cache_write
    }

    /// Adds `other` to these counts.
    pub const fn add(&mut self, other: &Self) {
        self.input += other.input;
        self.output += other.output;
        self.cache_read += other.cache_read;
        self.cache_write += other.cache_write;
    }
}

/// One coding session, as read from its source.
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    /// Where the session was recorded.
    pub source: Source,
    /// When the session started (UTC).
    pub started: DateTime<Utc>,
    /// Project directory, when the source records one.
    pub project: Option<String>,
    /// Model name, when the source records one.
    pub model: Option<String>,
    /// Token counts.
    pub tokens: Tokens,
    /// Cost in USD: the source's recorded cost if it has one, else an estimate from the price table. `None` when neither exists.
    pub cost_usd: Option<f64>,
}

/// Sessions found by one reader, plus how many candidate entries were skipped as unreadable.
#[derive(Debug, Default)]
pub struct Read {
    /// Sessions that were read.
    pub sessions: Vec<Session>,
    /// Entries that looked like sessions but could not be read.
    pub skipped: usize,
}

/// Every file under `root` whose name ends with `suffix`, searched recursively.
///
/// # Errors
///
/// Returns an error when `root` or a directory under it cannot be read.
pub fn files_with_suffix(root: &Path, suffix: &str) -> io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(suffix))
            {
                found.push(path);
            }
        }
    }
    found.sort();
    Ok(found)
}

/// Adds one source's sessions to `sessions`. Returns a warning when the source could not be read or skipped entries.
pub fn add_source<E: fmt::Display>(
    name: &str,
    result: Result<Read, E>,
    sessions: &mut Vec<Session>,
) -> Option<String> {
    match result {
        Ok(read) => {
            sessions.extend(read.sessions);
            (read.skipped > 0)
                .then(|| format!("{name}: skipped {} unreadable entries", read.skipped))
        }
        Err(err) => Some(format!("{name}: {err}")),
    }
}

/// Parses an RFC 3339 timestamp into UTC. Returns `None` for any other text.
#[must_use]
pub fn parse_timestamp(text: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|time| time.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::testing::ScratchDir;

    #[test]
    fn tokens_total_and_add() {
        let mut tokens = Tokens {
            input: 1,
            output: 2,
            cache_read: 3,
            cache_write: 4,
        };
        assert_eq!(tokens.total(), 10);
        tokens.add(&Tokens {
            input: 10,
            output: 20,
            cache_read: 30,
            cache_write: 40,
        });
        assert_eq!(
            tokens,
            Tokens {
                input: 11,
                output: 22,
                cache_read: 33,
                cache_write: 44
            }
        );
    }

    fn one_session() -> Session {
        Session {
            source: Source::Agy,
            started: parse_timestamp("2026-10-25T00:00:00Z").unwrap(),
            project: None,
            model: None,
            tokens: Tokens::default(),
            cost_usd: None,
        }
    }

    #[test]
    fn a_readable_source_adds_its_sessions_and_warns_only_about_skips() {
        let mut sessions = Vec::new();
        let clean = Read {
            sessions: vec![one_session()],
            skipped: 0,
        };
        assert_eq!(
            add_source::<io::Error>("agy", Ok(clean), &mut sessions),
            None
        );
        assert_eq!(sessions.len(), 1);

        let skipped = Read {
            sessions: vec![one_session()],
            skipped: 2,
        };
        assert_eq!(
            add_source::<io::Error>("agy", Ok(skipped), &mut sessions),
            Some("agy: skipped 2 unreadable entries".to_owned())
        );
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn an_unreadable_source_adds_nothing_and_names_itself() {
        let mut sessions = Vec::new();
        let err = io::Error::other("no such folder");
        assert_eq!(
            add_source("droid", Err(err), &mut sessions),
            Some("droid: no such folder".to_owned())
        );
        assert_eq!(sessions, Vec::<Session>::new());
    }

    #[test]
    fn timestamps_accept_offsets_and_reject_text() {
        assert_eq!(
            parse_timestamp("2026-10-05T17:04:35+02:00"),
            parse_timestamp("2026-10-05T15:04:35Z")
        );
        assert!(parse_timestamp("yesterday").is_none());
    }

    #[test]
    fn files_are_found_recursively_by_suffix() {
        let root = ScratchDir::new("files-by-suffix");
        let nested = root.join("a").join("b");
        fs::create_dir_all(&nested).unwrap();
        fs::write(root.join("one.settings.json"), "{}").unwrap();
        fs::write(nested.join("two.settings.json"), "{}").unwrap();
        fs::write(nested.join("three.jsonl"), "").unwrap();

        let found = files_with_suffix(&root, ".settings.json");

        let mut names: Vec<String> = found
            .unwrap()
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, ["one.settings.json", "two.settings.json"]);
    }

    #[test]
    fn missing_root_is_an_error() {
        assert!(files_with_suffix(Path::new("/nonexistent-droid-usage-root"), ".jsonl").is_err());
    }
}
