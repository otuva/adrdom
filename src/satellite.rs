//! A repository that holds code and tests and no record: the path back to
//! its primary, and the primary's list of satellites (per "Does install take
//! the path of a satellite?" and "What does the primary hold of a
//! satellite?").

use crate::out::Event;
use crate::repo;
use std::fs;
use std::path::{Path, PathBuf};

/// Where a command runs: the git root it is in, and the root of the record
/// it reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    /// The git root of the working directory.
    pub code: PathBuf,
    /// The canonical primary root in a satellite, else the same as `code`.
    pub record: PathBuf,
    /// The line of `adrdom/primary` as written, in a satellite.
    pub primary: Option<String>,
}

impl Place {
    pub fn is_satellite(&self) -> bool {
        self.primary.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Located {
    NotARepo,
    Here(Place),
    /// A satellite whose primary has no record at the path it holds.
    RecordMissing {
        code: PathBuf,
        primary: String,
    },
}

/// Whether a record sits at `root`.
pub fn has_record(root: &Path) -> bool {
    root.join(repo::LEDGER_PATH).exists()
}

/// The path `adrdom/primary` holds in the repository at `code_root`, as
/// written and trimmed; `None` when the file is missing or empty.
pub fn primary_of(code_root: &Path) -> Option<String> {
    let text = fs::read_to_string(code_root.join(repo::PRIMARY_PATH)).ok()?;
    let line = text.trim();
    (!line.is_empty()).then(|| line.to_string())
}

/// The place of the working directory.
pub fn locate() -> Located {
    let Some(code) = repo::find_root() else {
        return Located::NotARepo;
    };
    let Some(primary) = primary_of(&code) else {
        return Located::Here(Place {
            record: code.clone(),
            code,
            primary: None,
        });
    };
    match fs::canonicalize(code.join(&primary)) {
        Ok(record) if has_record(&record) => Located::Here(Place {
            code,
            record,
            primary: Some(primary),
        }),
        _ => Located::RecordMissing { code, primary },
    }
}

/// One satellite of a primary: its path from the primary's root and the url
/// of its origin ("" when it has none).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub path: String,
    pub url: String,
}

fn parse(text: &str) -> Vec<Row> {
    text.lines()
        // A tracked file may be checked out with CRLF line ends.
        .map(|l| l.trim_end_matches('\r'))
        .filter(|l| !l.trim().is_empty())
        .map(|l| match l.split_once('\t') {
            Some((p, u)) => Row {
                path: p.to_string(),
                url: u.to_string(),
            },
            None => Row {
                path: l.to_string(),
                url: String::new(),
            },
        })
        .collect()
}

/// The satellites the primary at `primary_root` lists, in file order.
pub fn rows(primary_root: &Path) -> Vec<Row> {
    fs::read_to_string(primary_root.join(repo::SATELLITES_PATH))
        .map(|t| parse(&t))
        .unwrap_or_default()
}

fn save(primary_root: &Path, rows: &[Row]) -> Result<Event, String> {
    let mut text = String::new();
    for r in rows {
        text.push_str(&format!("{}\t{}\n", r.path, r.url));
    }
    let path = primary_root.join(repo::SATELLITES_PATH);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(Event::FileWritten {
        path: repo::SATELLITES_PATH.to_string(),
    })
}

/// Adds `row`, or rewrites the url of the row with the same path in place;
/// no event when the list already holds it.
pub fn write_row(primary_root: &Path, row: Row) -> Result<Option<Event>, String> {
    let mut all = rows(primary_root);
    match all.iter_mut().find(|r| r.path == row.path) {
        Some(r) if r.url == row.url => return Ok(None),
        Some(r) => r.url = row.url,
        None => all.push(row),
    }
    save(primary_root, &all).map(Some)
}

/// Drops the row of `path`; no event when there is none.
pub fn drop_row(primary_root: &Path, path: &str) -> Result<Option<Event>, String> {
    let mut all = rows(primary_root);
    let before = all.len();
    all.retain(|r| r.path != path);
    if all.len() == before {
        return Ok(None);
    }
    save(primary_root, &all).map(Some)
}

/// The rows whose satellite has no git checkout at its path.
pub fn missing(primary_root: &Path) -> Vec<Row> {
    rows(primary_root)
        .into_iter()
        .filter(|r| !primary_root.join(&r.path).join(".git").exists())
        .collect()
}

/// Whether a command may build decisions here: not in a satellite, and not
/// in a primary while a satellite of its list is missing (`missing` is
/// `missing(primary_root)`, read once by the caller).
pub fn builds(in_satellite: bool, missing: &[Row]) -> bool {
    !in_satellite && missing.is_empty()
}

/// The absolute, canonical form of `given`, a path as typed on the command
/// line and resolved against the current directory. A path that does not
/// exist yet is canonical up to its last component. A failure is returned,
/// never a relative path.
pub fn absolute(given: &Path) -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let abs = cwd.join(given);
    match fs::canonicalize(&abs) {
        Ok(p) => Ok(p),
        Err(e) => match (abs.parent(), abs.file_name()) {
            (Some(dir), Some(name)) => fs::canonicalize(dir)
                .map(|d| d.join(name))
                .map_err(|_| e.to_string()),
            _ => Err(e.to_string()),
        },
    }
}

/// `url` without the userinfo of an http or https url (`user:token@`); any
/// other url is returned as it is.
pub fn without_userinfo(url: &str) -> String {
    let lower = url.to_ascii_lowercase();
    let Some(scheme) = ["http://", "https://"]
        .into_iter()
        .find(|s| lower.starts_with(s))
    else {
        return url.to_string();
    };
    let rest = &url[scheme.len()..];
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    match rest[..end].rfind('@') {
        Some(at) => format!("{}{}", &url[..scheme.len()], &rest[at + 1..]),
        None => url.to_string(),
    }
}

/// The path from `from` to `to`, both canonical and absolute, `/`
/// separated; `.` when they are the same.
pub fn relative_path(from: &Path, to: &Path) -> String {
    let a: Vec<_> = from.components().collect();
    let b: Vec<_> = to.components().collect();
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let mut parts: Vec<String> = vec!["..".to_string(); a.len() - common];
    parts.extend(
        b[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().to_string()),
    );
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths() {
        let p = Path::new;
        assert_eq!(relative_path(p("/a/b"), p("/a/b")), ".");
        assert_eq!(relative_path(p("/a/b"), p("/a/c")), "../c");
        assert_eq!(relative_path(p("/a/b"), p("/a/b/c/d")), "c/d");
        assert_eq!(relative_path(p("/a/b/c"), p("/a")), "../..");
    }

    #[test]
    fn crlf_lines_parse_like_lf_lines() {
        let rows = parse("../a\r\n\r\n../b\thttps://h/o/r.git\r\n");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].path, "../a");
        assert_eq!(rows[0].url, "");
        assert_eq!(rows[1].path, "../b");
        assert_eq!(rows[1].url, "https://h/o/r.git");
        let dir = std::env::temp_dir().join(format!("adr-sat-crlf-{}", std::process::id()));
        fs::create_dir_all(dir.join("adrdom")).unwrap();
        fs::write(dir.join(repo::PRIMARY_PATH), "../primary\r\n").unwrap();
        assert_eq!(primary_of(&dir).as_deref(), Some("../primary"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn userinfo_is_dropped_from_http_urls_only() {
        assert_eq!(
            without_userinfo("https://user:TOKEN@host/o/r.git"),
            "https://host/o/r.git"
        );
        assert_eq!(
            without_userinfo("http://TOKEN@host:8080/r"),
            "http://host:8080/r"
        );
        assert_eq!(without_userinfo("HTTPS://u:p@host/r"), "HTTPS://host/r");
        assert_eq!(
            without_userinfo("https://host/o/r@2.git"),
            "https://host/o/r@2.git"
        );
        assert_eq!(
            without_userinfo("https://host/o/r.git"),
            "https://host/o/r.git"
        );
        assert_eq!(
            without_userinfo("ssh://git@host/o/r.git"),
            "ssh://git@host/o/r.git"
        );
        assert_eq!(without_userinfo("git@host:o/r.git"), "git@host:o/r.git");
        assert_eq!(without_userinfo(""), "");
    }

    #[test]
    fn builds_needs_a_primary_with_no_missing_satellite() {
        let row = Row {
            path: "../a".into(),
            url: String::new(),
        };
        assert!(builds(false, &[]));
        assert!(!builds(true, &[]));
        assert!(!builds(false, &[row]));
    }

    #[test]
    fn rows_are_lenient_and_written_once_per_path() {
        let dir = std::env::temp_dir().join(format!("adr-sat-unit-{}", std::process::id()));
        fs::create_dir_all(dir.join("adrdom/state")).unwrap();
        fs::write(dir.join(repo::SATELLITES_PATH), "../a\n\n../b\tu\n").unwrap();
        assert_eq!(rows(&dir).len(), 2);
        assert_eq!(rows(&dir)[0].url, "");
        assert!(write_row(
            &dir,
            Row {
                path: "../b".into(),
                url: "u".into()
            }
        )
        .unwrap()
        .is_none());
        write_row(
            &dir,
            Row {
                path: "../a".into(),
                url: "x".into(),
            },
        )
        .unwrap();
        assert_eq!(rows(&dir)[0].url, "x");
        assert_eq!(rows(&dir).len(), 2);
        assert!(drop_row(&dir, "../zz").unwrap().is_none());
        drop_row(&dir, "../a").unwrap();
        assert_eq!(rows(&dir).len(), 1);
        assert_eq!(missing(&dir).len(), 1);
        fs::remove_dir_all(&dir).unwrap();
    }
}
