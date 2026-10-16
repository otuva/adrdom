//! The session an agent program runs a command in: the identifier it gives
//! the session, and the role a session has taken (a planner or plain). With
//! no identifier nothing here does anything (a command does not stop when
//! there is no session identifier, 8081c82d).
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// The environment variables an agent program sets for its session, first
/// found wins. Only Claude Code's is verified.
/// adr-code: e53b805c-9272-4b89-985d-61426fb5fa48
pub const SESSION_VARS: &[&str] = &["CLAUDE_CODE_SESSION_ID"];

/// What a session is to the tool: plain, or a planner (the word the role
/// file holds beside its identifier).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Role {
    #[default]
    Plain,
    Planner,
}

const PLANNER_WORD: &str = "planner";

/// The identifier the agent program gives the session, read from the
/// environment the command runs in: the first variable of `SESSION_VARS`
/// with a non-empty value. A value holding a tab, CR or newline counts as
/// absent, for the role file is tab and line separated.
pub fn id() -> Option<String> {
    SESSION_VARS
        .iter()
        .find_map(|name| usable(std::env::var(name).ok()?))
}

fn usable(value: String) -> Option<String> {
    (!value.is_empty() && !value.contains(['\t', '\r', '\n'])).then_some(value)
}

type Cache = Mutex<HashMap<(PathBuf, String), Role>>;

fn cache() -> &'static Cache {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// The role of the current session in the repository at `root`; a roles
/// file that cannot be read counts as plain here (see `role_checked`).
pub fn role(root: &Path) -> Role {
    role_checked(root).unwrap_or(Role::Plain)
}

/// `role`, but a roles file that exists and cannot be read is an error. No
/// identifier or no file is plain. The file is read as bytes and the lines
/// matched as bytes, so a valid line beside invalid UTF-8 still counts.
pub fn role_checked(root: &Path) -> std::io::Result<Role> {
    match id() {
        Some(id) => role_of(root, &id),
        None => Ok(Role::Plain),
    }
}

/// `role_checked` for session `id`, read once per repository and session
/// for the process (a success only). The file is read from disk, never
/// through `repo::Source`: it is untracked.
fn role_of(root: &Path, id: &str) -> std::io::Result<Role> {
    let key = (root.to_path_buf(), id.to_string());
    if let Some(found) = cache().lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
        return Ok(*found);
    }
    let line = format!("{}\t{}", id, PLANNER_WORD);
    let found = match std::fs::read(root.join(crate::repo::ROLES_PATH)) {
        Ok(bytes) => {
            // A line edited to CRLF still reads.
            let held = bytes
                .split(|b| *b == b'\n')
                .any(|l| l.strip_suffix(b"\r").unwrap_or(l) == line.as_bytes());
            if held {
                Role::Planner
            } else {
                Role::Plain
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Role::Plain,
        Err(e) => return Err(e),
    };
    cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(key, found);
    Ok(found)
}

/// Marks the current session a planner. Does nothing when there is no
/// session identifier or the session is already marked. Nothing removes a
/// line: the role ends with the session and the line stays on file
/// (1ee664a1). Prints nothing: the role file is no ledger event.
pub fn mark_planner(root: &Path) -> std::io::Result<()> {
    match id() {
        Some(id) => mark_planner_as(root, &id),
        None => Ok(()),
    }
}

fn mark_planner_as(root: &Path, id: &str) -> std::io::Result<()> {
    if role_of(root, id)? == Role::Planner {
        return Ok(());
    }
    let path = root.join(crate::repo::ROLES_PATH);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    // A file whose last line lost its newline gets one before the new line.
    let lead = if std::fs::read(&path)?.last().is_some_and(|b| *b != b'\n') {
        "\n"
    } else {
        ""
    };
    file.write_all(format!("{}{}\t{}\n", lead, id, PLANNER_WORD).as_bytes())?;
    cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert((root.to_path_buf(), id.to_string()), Role::Planner);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("adr-session-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_value_with_a_separator_or_nothing_is_no_identifier() {
        assert_eq!(usable("abc".into()), Some("abc".into()));
        assert_eq!(usable(String::new()), None);
        assert_eq!(usable("a\tb".into()), None);
        assert_eq!(usable("a\r".into()), None);
        assert_eq!(usable("a\nb".into()), None);
    }

    #[test]
    fn marking_appends_once_and_reads_back() {
        let dir = scratch("mark");
        assert_eq!(role_of(&dir, "s1").unwrap(), Role::Plain);
        mark_planner_as(&dir, "s1").unwrap();
        mark_planner_as(&dir, "s1").unwrap();
        assert_eq!(role_of(&dir, "s1").unwrap(), Role::Planner);
        assert_eq!(role_of(&dir, "s2").unwrap(), Role::Plain);
        mark_planner_as(&dir, "s2").unwrap();
        let content = std::fs::read_to_string(dir.join(crate::repo::ROLES_PATH)).unwrap();
        assert_eq!(content, "s1\tplanner\ns2\tplanner\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_file_is_read_from_disk() {
        let dir = scratch("disk");
        std::fs::create_dir_all(dir.join(crate::repo::STATE_DIR)).unwrap();
        std::fs::write(
            dir.join(crate::repo::ROLES_PATH),
            "other\tplanner\ns3\tplanner\n",
        )
        .unwrap();
        assert_eq!(role_of(&dir, "s3").unwrap(), Role::Planner);
        assert_eq!(role_of(&dir, "s").unwrap(), Role::Plain);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
