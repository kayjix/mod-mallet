//! Checks commit messages against the Conventional Commits format.
//!
//! The format is `type(scope): subject`, with the scope optional. Release
//! notes are later generated from these messages, so the type has to be one
//! of a fixed set.

use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::tools;

const TYPES: &[&str] = &[
    "feat", "fix", "docs", "refactor", "test", "perf", "build", "ci", "chore", "revert",
];

const MAX_SUBJECT_LINE: usize = 100;

/// Checks the message in `file`, or, without one, the message of the
/// commit being made.
pub(crate) fn run(file: Option<PathBuf>) -> Result<()> {
    let file = match file {
        Some(file) => file,
        None => being_committed()?,
    };
    let message = std::fs::read_to_string(&file)
        .with_context(|| format!("could not read `{}`", file.display()))?;
    check(&message)
}

/// The file git writes the message of the commit being made to, in this
/// checkout or worktree.
fn being_committed() -> Result<PathBuf> {
    let mut git = Command::new("git");
    git.args(["rev-parse", "--git-path", "COMMIT_EDITMSG"]);
    let path = tools::capture(git, "git rev-parse")?;
    Ok(PathBuf::from(path.trim()))
}

fn check(message: &str) -> Result<()> {
    let first_line = message
        .lines()
        .find(|line| !line.starts_with('#'))
        .unwrap_or("")
        .trim_end();

    // Messages git writes itself.
    if first_line.starts_with("Merge ") || first_line.starts_with("Revert \"") {
        return Ok(());
    }

    let help = format!(
        "expected `type(scope): subject` with type one of: {}",
        TYPES.join(", ")
    );
    let Some((head, subject)) = first_line.split_once(": ") else {
        bail!("commit message `{first_line}` has no `type: subject`; {help}");
    };
    let head = head.strip_suffix('!').unwrap_or(head);
    let commit_type = match head.split_once('(') {
        Some((commit_type, scope)) => {
            let Some(scope) = scope.strip_suffix(')') else {
                bail!("the scope in `{head}` is not closed; {help}");
            };
            if scope.is_empty() {
                bail!("the scope in `{head}` is empty; {help}");
            }
            commit_type
        }
        None => head,
    };
    if !TYPES.contains(&commit_type) {
        bail!("`{commit_type}` is not a known commit type; {help}");
    }
    if subject.trim().is_empty() {
        bail!("the commit message has no subject; {help}");
    }
    if first_line.chars().count() > MAX_SUBJECT_LINE {
        bail!("the first line is longer than {MAX_SUBJECT_LINE} characters");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_conventional_messages() {
        for message in [
            "feat: report plugins that failed to load",
            "fix(skse-log): accept names with brackets",
            "refactor(core)!: rename Finding::kind",
            "docs: explain the pipeline\n\nLonger body.",
            "# comment line first\nchore: update dependencies",
            "Merge branch 'main' into feature",
            "Revert \"feat: something\"",
        ] {
            assert!(check(message).is_ok(), "{message}");
        }
    }

    #[test]
    fn rejects_everything_else() {
        for message in [
            "",
            "Fixed stuff",
            "feature: unknown type",
            "feat:missing space",
            "feat: ",
            "fix(): empty scope",
            "fix(scope: not closed",
        ] {
            assert!(check(message).is_err(), "{message}");
        }
    }

    #[test]
    fn rejects_an_overlong_first_line() {
        let message = format!("feat: {}", "x".repeat(MAX_SUBJECT_LINE));
        assert!(check(&message).is_err());
    }
}
