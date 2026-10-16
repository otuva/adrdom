//! The rule for what a blame file's origin may be, and the plain form of an origin path.
use crate::repo;
use std::path::Path;

/// Whether `path` (repo-root-relative, forward slashes) may name an origin:
/// not under `repo::ADRDOM_DIR`, not a directory in `source` and not a
/// symbolic link (per the "origin" term: "a prose file outside adrdom whose
/// text the records take in" -- `adrdom/blame`'s own tree is never outside
/// adrdom, and neither a directory nor a link is a file; `Source::is_symlink`
/// sees a link in the working tree only). A path that simply does not exist
/// at all is not this rule's problem: that is the missing-origin case
/// (0c3d498a A), checked on its own. Read by `file_state`, so `approve`,
/// `check` and `show` hold the rule in exactly one place.
pub fn valid_origin_path(root: &Path, source: &repo::Source, path: &str) -> bool {
    if leaves_repository(path) {
        return false;
    }
    let plain = plain_path(path);
    let under_adrdom =
        plain == repo::ADRDOM_DIR || plain.starts_with(&format!("{}/", repo::ADRDOM_DIR));
    !under_adrdom && !source.is_dir(root, &plain) && !source.is_symlink(root, &plain)
}

/// Whether the text of `path` cannot name a file inside the repository:
/// absolute, ending in `/`, or with a `..` component that climbs above the
/// repository root. Text only: nothing is read from disk.
fn leaves_repository(path: &str) -> bool {
    let path = path.trim();
    if path.starts_with('/') || path.ends_with('/') {
        return true;
    }
    let mut depth = 0usize;
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if depth == 0 {
                    return true;
                }
                depth -= 1;
            }
            _ => depth += 1,
        }
    }
    false
}

/// `path` with every empty and `.` component dropped and every `..`
/// resolved against the component before it, so two spellings of one
/// path (`docs/x.md`, `./docs/x.md`, `docs/../docs/x.md`) compare equal.
/// Text only: nothing is read from disk.
pub fn plain_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.trim().split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            p => parts.push(p),
        }
    }
    parts.join("/")
}
