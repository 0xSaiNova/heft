//! Git recency signal for activity detection.
//!
//! Parses .git/logs/HEAD (the reflog) for the most recent operation timestamp.
//! Falls back to .git/index mtime for shallow clones or unreadable reflogs.

use std::path::Path;
use std::time::{Duration, SystemTime};

/// Returns the time of the most recent git activity in a project root.
/// Checks the reflog first, then falls back to .git/index mtime.
/// Returns None if the directory is not a git repo or both checks fail.
pub fn last_activity(project_root: &Path) -> Option<SystemTime> {
    let git_dir = project_root.join(".git");
    if !git_dir.is_dir() {
        return None;
    }

    // try reflog first: .git/logs/HEAD has an entry for every ref-changing operation
    if let Some(time) = parse_reflog_timestamp(&git_dir) {
        return Some(time);
    }

    // fallback: .git/index gets touched on checkout, commit, merge, rebase, stash
    std::fs::metadata(git_dir.join("index"))
        .ok()
        .and_then(|m| m.modified().ok())
}

/// Parse the most recent timestamp from .git/logs/HEAD.
/// Reflog format: <old_sha> <new_sha> <name> <email> <timestamp> <tz>\t<message>
/// The timestamp field is a unix epoch seconds value.
fn parse_reflog_timestamp(git_dir: &Path) -> Option<SystemTime> {
    let reflog_path = git_dir.join("logs").join("HEAD");
    let content = std::fs::read_to_string(&reflog_path).ok()?;

    // get the last non-empty line
    let last_line = content.lines().rev().find(|l| !l.is_empty())?;

    // find the tab separator that precedes the message
    let before_tab = last_line.split('\t').next()?;

    // the timestamp is the second-to-last whitespace-delimited field before the tab
    // format: ... <name> <email> <timestamp> <tz>
    let fields: Vec<&str> = before_tab.split_whitespace().collect();
    if fields.len() < 2 {
        return None;
    }

    // timestamp is at fields[len - 2], timezone at fields[len - 1]
    let ts_str = fields[fields.len() - 2];
    let ts: u64 = ts_str.parse().ok()?;

    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(ts))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parse_reflog_extracts_timestamp() {
        let tmp = tempfile::tempdir().unwrap();
        let git_dir = tmp.path().join(".git");
        fs::create_dir_all(git_dir.join("logs")).unwrap();

        // write a realistic reflog entry
        fs::write(
            git_dir.join("logs").join("HEAD"),
            "0000000 abc1234 John Doe <john@example.com> 1700000000 +0000\tcommit: initial\n\
             abc1234 def5678 John Doe <john@example.com> 1700100000 +0000\tcommit: second\n",
        )
        .unwrap();

        // also create an index file so the function can find .git
        fs::write(git_dir.join("index"), "").unwrap();

        let result = last_activity(tmp.path());
        assert!(result.is_some());

        let epoch = result
            .unwrap()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        assert_eq!(epoch.as_secs(), 1700100000);
    }

    #[test]
    fn falls_back_to_index_mtime() {
        let tmp = tempfile::tempdir().unwrap();
        let git_dir = tmp.path().join(".git");
        fs::create_dir_all(&git_dir).unwrap();

        // no reflog, but index exists
        fs::write(git_dir.join("index"), "fake index").unwrap();

        let result = last_activity(tmp.path());
        assert!(result.is_some());
    }

    #[test]
    fn returns_none_for_non_git_dir() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(last_activity(tmp.path()).is_none());
    }
}
