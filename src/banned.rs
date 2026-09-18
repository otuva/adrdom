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

pub fn read_set(path: &Path) -> HashSet<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_string())
        .collect()
}

pub fn append(path: &Path, hashes: &[String]) -> std::io::Result<()> {
    if hashes.is_empty() {
        return Ok(());
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    for h in hashes {
        writeln!(f, "{}", h)?;
    }
    Ok(())
}
