//! The neighbours file: the record that the heading step of a question was
//! done, and the neighbours it brought.
//!
//! "Does the reader read a neighbour of a heading?"
//! (60341be0-fd05-4281-92d7-668584a8539a A): no. The tool brings the ten
//! nearest current decisions and term definitions of a heading by embedding
//! and records them in `adrdom/state/neighbours.tsv`, an untracked file: one
//! row for each neighbour, the heading hash (`record::heading_hash`) and the
//! neighbour's identifier, in the order of nearness. Recording a heading
//! again replaces its rows, so the latest set is the one that counts. When it
//! records, the tool first drops each row whose heading is in no topic file.
use crate::msg;
use crate::reader::Live;
use crate::repo;
use std::collections::HashMap;
use std::path::Path;

/// The neighbour identifiers on record for each heading hash. A line that is
/// not two columns is not a row and is left out.
pub fn read(root: &Path) -> HashMap<String, Vec<String>> {
    parse(&std::fs::read_to_string(root.join(repo::NEIGHBOURS_PATH)).unwrap_or_default())
}

fn parse(content: &str) -> HashMap<String, Vec<String>> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for line in content.lines() {
        let p: Vec<&str> = line.split('\t').collect();
        if let [hash, id] = p[..] {
            if !hash.is_empty() && !id.is_empty() {
                out.entry(hash.to_string()).or_default().push(id.to_string());
            }
        }
    }
    out
}

/// Records `entries`, each a heading hash and the identifiers of its
/// neighbours: the rows of a heading already on record are replaced, and the
/// rows of a heading in no topic file are dropped. The file is locked for the
/// read and the write, and written in place, so two sessions that record at
/// once keep each other's rows.
pub fn record(root: &Path, entries: &[(String, Vec<String>)], live: &Live) -> Result<(), String> {
    use std::io::{Read, Seek, Write};
    if entries.is_empty() {
        return Ok(());
    }
    // The records are read before the lock is taken, not under it.
    live.has_text("");
    let path = root.join(repo::NEIGHBOURS_PATH);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&path)
        .map_err(msg::neighbours_write_failed)?;
    file.lock().map_err(msg::neighbours_write_failed)?;
    let mut old = String::new();
    // Bytes that are no text read as no rows.
    if file.read_to_string(&mut old).is_err() {
        old.clear();
    }
    let mut text = String::new();
    for line in old.lines() {
        let hash = line.split('\t').next().unwrap_or("");
        if line.split('\t').count() == 2 && live.has_text(hash) && !entries.iter().any(|(h, _)| h == hash) {
            text.push_str(line);
            text.push('\n');
        }
    }
    for (hash, ids) in entries {
        for id in ids {
            text.push_str(&format!("{}\t{}\n", hash, id));
        }
    }
    file.set_len(0)
        .and_then(|_| file.rewind())
        .and_then(|_| file.write_all(text.as_bytes()))
        .map_err(msg::neighbours_write_failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_group_by_heading_in_order_and_a_bad_line_is_no_row() {
        let m = parse("h1\ta\nh2\tb\nbad\nh1\tc\nh3\t\n");
        assert_eq!(m["h1"], vec!["a".to_string(), "c".to_string()]);
        assert_eq!(m["h2"], vec!["b".to_string()]);
        assert_eq!(m.len(), 2);
    }

    // Two sessions that record at once keep each other's rows.
    #[test]
    fn records_made_at_once_all_stay() {
        let root = std::env::temp_dir().join(format!("adr-neighbours-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("adrdom/decisions")).unwrap();
        std::fs::create_dir_all(root.join("adrdom/state")).unwrap();
        let mut topic = String::from("# Topic\n\nProse.\n");
        for i in 0..16 {
            topic.push_str(&format!("\n### Does thing {} hold?\n", i));
        }
        std::fs::write(root.join("adrdom/decisions/topic.md"), topic).unwrap();
        let (files, errors) = crate::record::load_all_via(&root, &repo::Source::WorkingTree);
        assert!(errors.is_empty(), "{:?}", errors);
        let hashes: Vec<String> = files[0].sections.iter().map(crate::record::heading_hash).collect();
        assert_eq!(hashes.len(), 16);
        std::thread::scope(|s| {
            for h in &hashes {
                let root = &root;
                s.spawn(move || {
                    record(root, &[(h.clone(), vec!["a".to_string(), "b".to_string()])], &Live::of(root)).unwrap();
                });
            }
        });
        let on_record = read(&root);
        for h in &hashes {
            assert_eq!(on_record.get(h).map(Vec::len), Some(2), "{}", h);
        }
        std::fs::remove_dir_all(&root).unwrap();
    }
}
