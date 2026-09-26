use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Same tokenization for `ban` and `check --banned`: the whole trimmed,
/// lowercased line, plus each maximal run of Unicode alphanumerics in it.
pub fn tokenize(line: &str) -> Vec<String> {
    let trimmed = line.trim().to_lowercase();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let mut tokens = vec![trimmed.clone()];
    let mut current = String::new();
    for c in trimmed.chars() {
        if c.is_alphanumeric() {
            current.push(c);
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// Parses one line of `adrdom/state/banned.sha256`: the hash, and the term
/// uuid carried alongside it when the word was banned with `--term`.
fn parse_line(line: &str) -> (String, Option<String>) {
    let line = line.trim();
    match line.split_once(' ') {
        Some((hash, uuid)) => (hash.to_string(), Some(uuid.trim().to_string())),
        None => (line.to_string(), None),
    }
}

/// Formats one line of `adrdom/state/banned.sha256`: `<hash>` alone, or
/// `<hash> <uuid>` when a term uuid is carried.
pub fn format_line(hash: &str, uuid: &Option<String>) -> String {
    match uuid {
        Some(u) => format!("{} {}", hash, u),
        None => hash.to_string(),
    }
}

/// Every hash in the ban list, dropping the term uuid column.
pub fn read_set(path: &Path) -> HashSet<String> {
    read_entries(path).into_iter().map(|(h, _)| h).collect()
}

/// Every line of the ban list, in file order: the hash, and the term uuid
/// on the line, if any.
pub fn read_entries(path: &Path) -> Vec<(String, Option<String>)> {
    parse_entries(&fs::read_to_string(path).unwrap_or_default())
}

/// Parses ban-list entries straight from `content`, without touching the
/// filesystem: what `read_entries` does after its own
/// `fs::read_to_string`, split out so a caller reading through
/// `repo::Source` (`check`, so a staged edit to the ban list is judged
/// too) doesn't have to write the bytes to a temp file first.
pub fn parse_entries(content: &str) -> Vec<(String, Option<String>)> {
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(parse_line)
        .collect()
}

pub fn append(path: &Path, entries: &[(String, Option<String>)]) -> std::io::Result<()> {
    if entries.is_empty() {
        return Ok(());
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    for (hash, uuid) in entries {
        writeln!(f, "{}", format_line(hash, uuid))?;
    }
    Ok(())
}
