use crate::record::{normalize, sha256_hex};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Term {
    pub name: String,
    pub id: Option<String>,
    pub definition: String,
    /// Part of the file shape (parsed and required) but not read anywhere
    /// yet; no view surfaces individual terms today.
    #[allow(dead_code)]
    pub test: String,
    pub owner: String,
    pub aliases: Vec<String>,
    pub banned: Vec<String>,
    pub retired: bool,
    /// 1-based line number of the `## name` heading.
    pub start_line: usize,
    /// 1-based line number of the `id: ` line, if present.
    pub id_line: Option<usize>,
}

pub struct TermsFile {
    pub path: PathBuf,
    pub lines: Vec<String>,
    pub terms: Vec<Term>,
    pub errors: Vec<String>,
}

fn kv(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{}:", key);
    line.strip_prefix(&prefix).map(|v| v.trim().to_string())
}

fn split_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

pub fn load(path: &Path) -> io::Result<TermsFile> {
    let content = fs::read_to_string(path)?;
    let lines = crate::record::split_lines(&content);
    let mut tf = parse(&lines);
    tf.path = path.to_path_buf();
    Ok(tf)
}

pub fn save(tf: &TermsFile) -> io::Result<()> {
    fs::write(&tf.path, tf.lines.join("\n"))
}

pub fn file_hash(tf: &TermsFile) -> String {
    sha256_hex(&normalize(&tf.lines.join("\n")))
}

pub fn parse(lines: &[String]) -> TermsFile {
    let mut errors = Vec::new();
    let mut terms: Vec<Term> = Vec::new();

    let h1_lines: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.trim_end() == "# Terms")
        .map(|(i, _)| i)
        .collect();
    if h1_lines.is_empty() {
        errors.push("line 1: missing h1 \"# Terms\"".to_string());
    } else {
        for &i in h1_lines.iter().skip(1) {
            errors.push(format!("line {}: duplicate h1 \"# Terms\"", i + 1));
        }
    }

    let mut heading_idxs: Vec<usize> = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        if l.starts_with("## ") && !l.starts_with("### ") {
            heading_idxs.push(i);
        }
    }

    let first_heading = heading_idxs.first().copied().unwrap_or(lines.len());
    for (i, l) in lines.iter().enumerate().take(first_heading) {
        if l.trim().is_empty() {
            continue;
        }
        if h1_lines.contains(&i) {
            continue;
        }
        errors.push(format!("line {}: unknown line", i + 1));
    }

    for (hi, &start) in heading_idxs.iter().enumerate() {
        let end = heading_idxs.get(hi + 1).copied().unwrap_or(lines.len());
        let name = lines[start]["## ".len()..].trim_end().to_string();

        let mut id: Option<String> = None;
        let mut id_line: Option<usize> = None;
        let mut definition: Option<String> = None;
        let mut test: Option<String> = None;
        let mut owner: Option<String> = None;
        let mut aliases: Option<String> = None;
        let mut banned: Option<String> = None;
        let mut retired = false;

        for (j, l) in lines.iter().enumerate().take(end).skip(start + 1) {
            if l.trim().is_empty() {
                continue;
            }
            if l.trim_end() == "# Terms" {
                // Already reported as a duplicate h1 above.
                continue;
            }
            if let Some(v) = kv(l, "id") {
                if id.is_none() {
                    id = Some(v);
                    id_line = Some(j);
                }
            } else if let Some(v) = kv(l, "definition") {
                if definition.is_none() {
                    definition = Some(v);
                }
            } else if let Some(v) = kv(l, "test") {
                if test.is_none() {
                    test = Some(v);
                }
            } else if let Some(v) = kv(l, "owner") {
                if owner.is_none() {
                    owner = Some(v);
                }
            } else if let Some(v) = kv(l, "aliases") {
                if aliases.is_none() {
                    aliases = Some(v);
                }
            } else if let Some(v) = kv(l, "banned") {
                if banned.is_none() {
                    banned = Some(v);
                }
            } else if let Some(v) = kv(l, "retired") {
                if v == "yes" {
                    retired = true;
                } else {
                    errors.push(format!("line {}: unknown line", j + 1));
                }
            } else {
                errors.push(format!("line {}: unknown line", j + 1));
            }
        }

        if id.is_none() {
            errors.push(format!(
                "line {}: missing id; owner runs adr approve adrdom/terms.md",
                start + 1
            ));
        }
        if definition.is_none() {
            errors.push(format!("line {}: missing field \"definition\"", start + 1));
        }
        if test.is_none() {
            errors.push(format!("line {}: missing field \"test\"", start + 1));
        }
        if owner.is_none() {
            errors.push(format!("line {}: missing field \"owner\"", start + 1));
        }
        if aliases.is_none() {
            errors.push(format!("line {}: missing field \"aliases\"", start + 1));
        }
        if banned.is_none() {
            errors.push(format!("line {}: missing field \"banned\"", start + 1));
        }

        terms.push(Term {
            name,
            id,
            definition: definition.unwrap_or_default(),
            test: test.unwrap_or_default(),
            owner: owner.unwrap_or_default(),
            aliases: split_csv(aliases.as_deref().unwrap_or("")),
            banned: split_csv(banned.as_deref().unwrap_or("")),
            retired,
            start_line: start + 1,
            id_line: id_line.map(|x| x + 1),
        });
    }

    let mut seen_ids: HashMap<String, usize> = HashMap::new();
    for t in &terms {
        if let Some(id) = &t.id {
            let line = t.id_line.unwrap_or(t.start_line);
            if let Some(&prev_line) = seen_ids.get(id) {
                errors.push(format!(
                    "line {}: duplicate id \"{}\" (also used at line {})",
                    line, id, prev_line
                ));
            } else {
                seen_ids.insert(id.clone(), line);
            }
        }
    }

    errors.extend(duplicate_word_errors(&terms));

    TermsFile {
        path: PathBuf::new(),
        lines: lines.to_vec(),
        terms,
        errors,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WordKind {
    NameAlias,
    Banned,
}

fn duplicate_word_errors(terms: &[Term]) -> Vec<String> {
    let mut occ: HashMap<String, Vec<(usize, WordKind)>> = HashMap::new();
    for (idx, t) in terms.iter().enumerate() {
        let mut namealias = vec![t.name.clone()];
        namealias.extend(t.aliases.iter().cloned());
        for w in namealias {
            let wl = w.trim().to_lowercase();
            if wl.is_empty() {
                continue;
            }
            occ.entry(wl).or_default().push((idx, WordKind::NameAlias));
        }
        for w in &t.banned {
            let wl = w.trim().to_lowercase();
            if wl.is_empty() {
                continue;
            }
            occ.entry(wl).or_default().push((idx, WordKind::Banned));
        }
    }

    let mut errors = Vec::new();
    let mut reported: BTreeSet<(String, usize, usize)> = BTreeSet::new();
    let mut words: Vec<&String> = occ.keys().collect();
    words.sort();
    for word in words {
        let occs = &occ[word];
        let mut entries: Vec<usize> = occs.iter().map(|(i, _)| *i).collect();
        entries.sort_unstable();
        entries.dedup();

        if entries.len() > 1 {
            for wi in 0..entries.len() {
                for wj in (wi + 1)..entries.len() {
                    let a = entries[wi];
                    let b = entries[wj];
                    let key = (word.clone(), a.min(b), a.max(b));
                    if reported.insert(key) {
                        errors.push(format!(
                            "line {}: word \"{}\" used in both \"{}\" (line {}) and \"{}\" (line {})",
                            terms[a].start_line.min(terms[b].start_line),
                            word,
                            terms[a].name,
                            terms[a].start_line,
                            terms[b].name,
                            terms[b].start_line,
                        ));
                    }
                }
            }
        } else if entries.len() == 1 {
            let e = entries[0];
            let has_namealias = occs
                .iter()
                .any(|(i, k)| *i == e && *k == WordKind::NameAlias);
            let has_banned = occs.iter().any(|(i, k)| *i == e && *k == WordKind::Banned);
            if has_namealias && has_banned {
                errors.push(format!(
                    "line {}: word \"{}\" used as both name/alias and banned in \"{}\"",
                    terms[e].start_line, word, terms[e].name,
                ));
            }
        }
    }
    errors
}

/// `owner` paths are checked separately from `parse` so parsing stays pure
/// (no filesystem access) and testable without a repo root.
pub fn check_owners(tf: &TermsFile, root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    for t in &tf.terms {
        if t.owner.is_empty() {
            // Already reported as a missing field.
            continue;
        }
        let p = root.join(&t.owner);
        if !p.exists() {
            errors.push(format!(
                "line {}: owner path \"{}\" does not exist",
                t.start_line, t.owner
            ));
        }
    }
    errors
}

/// Inserts `id: <uuid v4>` as the first line under each `## name` heading
/// that has none. Returns the number of ids inserted.
pub fn fill_missing_ids(tf: &mut TermsFile) -> usize {
    let mut targets: Vec<usize> = tf
        .terms
        .iter()
        .filter(|t| t.id.is_none())
        .map(|t| t.start_line) // 1-based heading line; also the 0-based insertion index.
        .collect();
    targets.sort_unstable();
    targets.reverse(); // Bottom-to-top so earlier insertions don't shift later indices.
    let count = targets.len();
    for start_line in targets {
        let new_line = format!("id: {}", Uuid::new_v4());
        tf.lines.insert(start_line, new_line);
    }
    let reparsed = parse(&tf.lines);
    tf.terms = reparsed.terms;
    tf.errors = reparsed.errors;
    count
}

fn is_boundary_ok(chars: &[char], start: usize, len: usize) -> bool {
    let before_ok = start == 0 || !chars[start - 1].is_alphanumeric();
    let after_ok = start + len == chars.len() || !chars[start + len].is_alphanumeric();
    before_ok && after_ok
}

/// Whole-word/phrase, case-insensitive, longest-phrase-first, non-overlapping
/// resolution of every term name/alias in `text`. Retired terms are included;
/// callers decide whether to refuse on a retired match. Entries without an id
/// are skipped. Returned pairs are unique and sorted by matched word.
pub fn resolve(text: &str, terms: &[Term]) -> Vec<(String, String)> {
    let mut phrases: Vec<(Vec<char>, String)> = Vec::new();
    for t in terms {
        let id = match &t.id {
            Some(i) => i.clone(),
            None => continue,
        };
        let mut names = vec![t.name.clone()];
        names.extend(t.aliases.iter().cloned());
        for n in names {
            let nl = n.trim().to_lowercase();
            if nl.is_empty() {
                continue;
            }
            phrases.push((nl.chars().collect(), id.clone()));
        }
    }
    // Longest phrase first; ties broken lexicographically for determinism.
    phrases.sort_by(|a, b| {
        b.0.len().cmp(&a.0.len()).then_with(|| {
            let sa: String = a.0.iter().collect();
            let sb: String = b.0.iter().collect();
            sa.cmp(&sb)
        })
    });

    let lower_text = text.to_lowercase();
    let chars: Vec<char> = lower_text.chars().collect();
    let n = chars.len();
    let mut results: BTreeSet<(String, String)> = BTreeSet::new();
    let mut i = 0;
    while i < n {
        let mut matched_len = 0;
        for (phrase, id) in &phrases {
            let plen = phrase.len();
            if plen == 0 || i + plen > n {
                continue;
            }
            if chars[i..i + plen] == phrase[..] && is_boundary_ok(&chars, i, plen) {
                let word: String = chars[i..i + plen].iter().collect();
                results.insert((word, id.clone()));
                matched_len = plen;
                break;
            }
        }
        if matched_len > 0 {
            i += matched_len;
        } else {
            i += 1;
        }
    }
    results.into_iter().collect()
}

/// Every banned-word hit for every entry, whole word, case-insensitive:
/// (1-based line number, banned word as it appears in `text` lowercased,
/// term name).
pub fn banned_hits(text: &str, terms: &[Term]) -> Vec<(usize, String, String)> {
    let mut hits = Vec::new();
    for (li, line) in text.split('\n').enumerate() {
        let lower_line = line.to_lowercase();
        let chars: Vec<char> = lower_line.chars().collect();
        let n = chars.len();
        for t in terms {
            for w in &t.banned {
                let wl = w.trim().to_lowercase();
                let pchars: Vec<char> = wl.chars().collect();
                let plen = pchars.len();
                if plen == 0 {
                    continue;
                }
                let mut i = 0;
                while i + plen <= n {
                    if chars[i..i + plen] == pchars[..] && is_boundary_ok(&chars, i, plen) {
                        let word: String = chars[i..i + plen].iter().collect();
                        hits.push((li + 1, word, t.name.clone()));
                        i += plen;
                    } else {
                        i += 1;
                    }
                }
            }
        }
    }
    hits
}

pub fn format_terms_line(pairs: &[(String, String)]) -> Option<String> {
    if pairs.is_empty() {
        return None;
    }
    let body = pairs
        .iter()
        .map(|(w, id)| format!("{}={}", w, id))
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!("terms: {}", body))
}

pub fn parse_terms_line(line: &str) -> Option<Vec<(String, String)>> {
    let rest = line.strip_prefix("terms: ")?;
    if rest.trim().is_empty() {
        return None;
    }
    let mut out = Vec::new();
    for part in rest.split(", ") {
        let (w, id) = part.split_once('=')?;
        out.push((w.to_string(), id.to_string()));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn lines_of(s: &str) -> Vec<String> {
        crate::record::split_lines(s)
    }

    fn well_formed() -> String {
        "# Terms\n\
\n\
## decision\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: A choice locked by the owner.\n\
test: Does the owner have to choose among options?\n\
owner: adrdom/decisions/plan.md\n\
aliases: choice\n\
banned: verdict\n\
\n\
## question\n\
id: 22222222-2222-2222-2222-222222222222\n\
definition: A thing the owner is asked to decide.\n\
test: Is it phrased as a question ending in a question mark?\n\
owner: adrdom/decisions/plan.md\n\
aliases: \n\
banned: \n\
"
        .to_string()
    }

    #[test]
    fn well_formed_file_parses_with_no_errors() {
        let tf = parse(&lines_of(&well_formed()));
        assert!(tf.errors.is_empty(), "unexpected errors: {:?}", tf.errors);
        assert_eq!(tf.terms.len(), 2);
        assert_eq!(tf.terms[0].name, "decision");
        assert_eq!(
            tf.terms[0].id.as_deref(),
            Some("11111111-1111-1111-1111-111111111111")
        );
        assert_eq!(tf.terms[0].aliases, vec!["choice".to_string()]);
        assert_eq!(tf.terms[0].banned, vec!["verdict".to_string()]);
        assert!(!tf.terms[0].retired);
        assert!(tf.terms[1].aliases.is_empty());
        assert!(tf.terms[1].banned.is_empty());
    }

    #[test]
    fn retired_line_is_recognized() {
        let mut s = well_formed();
        s = s.replacen(
            "banned: verdict\n",
            "banned: verdict\nretired: yes\n",
            1,
        );
        let tf = parse(&lines_of(&s));
        assert!(tf.errors.is_empty(), "unexpected errors: {:?}", tf.errors);
        assert!(tf.terms[0].retired);
    }

    #[test]
    fn missing_h1_is_a_shape_error() {
        let s = well_formed().replacen("# Terms\n", "", 1);
        let tf = parse(&lines_of(&s));
        assert!(tf.errors.iter().any(|e| e.contains("missing h1")));
    }

    #[test]
    fn duplicate_h1_is_a_shape_error() {
        let s = format!("# Terms\n{}", well_formed());
        let tf = parse(&lines_of(&s));
        assert!(tf.errors.iter().any(|e| e.contains("duplicate h1")));
    }

    #[test]
    fn missing_field_is_a_shape_error() {
        let s = well_formed().replacen("definition: A choice locked by the owner.\n", "", 1);
        let tf = parse(&lines_of(&s));
        assert!(tf
            .errors
            .iter()
            .any(|e| e.contains("missing field \"definition\"")));
    }

    #[test]
    fn unknown_line_is_a_shape_error() {
        let s = well_formed().replacen(
            "banned: verdict\n",
            "banned: verdict\nnotes: extra\n",
            1,
        );
        let tf = parse(&lines_of(&s));
        assert!(tf.errors.iter().any(|e| e.contains("unknown line")));
    }

    #[test]
    fn missing_id_gets_the_owner_wording() {
        let s = well_formed().replacen(
            "id: 11111111-1111-1111-1111-111111111111\n",
            "",
            1,
        );
        let tf = parse(&lines_of(&s));
        assert!(tf.errors.iter().any(|e| e
            .contains("missing id; owner runs adr approve adrdom/terms.md")));
    }

    #[test]
    fn duplicate_id_is_a_shape_error() {
        let s = well_formed().replacen(
            "22222222-2222-2222-2222-222222222222",
            "11111111-1111-1111-1111-111111111111",
            1,
        );
        let tf = parse(&lines_of(&s));
        assert!(tf.errors.iter().any(|e| e.contains("duplicate id")));
    }

    #[test]
    fn owner_path_that_does_not_exist_is_an_error() {
        let dir = std::env::temp_dir().join(format!("adrdom-terms-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let tf = parse(&lines_of(&well_formed()));
        let errs = check_owners(&tf, &dir);
        assert!(errs.iter().any(|e| e.contains("does not exist")));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn owner_path_that_exists_is_fine() {
        let dir = std::env::temp_dir().join(format!("adrdom-terms-test-{}", Uuid::new_v4()));
        fs::create_dir_all(dir.join("adrdom/decisions")).unwrap();
        fs::write(dir.join("adrdom/decisions/plan.md"), "# plan\n").unwrap();
        let tf = parse(&lines_of(&well_formed()));
        let errs = check_owners(&tf, &dir);
        assert!(errs.is_empty(), "unexpected: {:?}", errs);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn duplicate_word_across_two_entries_is_an_error() {
        let s = well_formed().replacen("aliases: choice\n", "aliases: question\n", 1);
        let tf = parse(&lines_of(&s));
        assert!(tf
            .errors
            .iter()
            .any(|e| e.contains("word \"question\"") && e.contains("decision") && e.contains("question")));
    }

    #[test]
    fn duplicate_word_as_name_and_banned_in_same_entry_is_an_error() {
        let s = well_formed().replacen("banned: verdict\n", "banned: decision\n", 1);
        let tf = parse(&lines_of(&s));
        assert!(tf
            .errors
            .iter()
            .any(|e| e.contains("word \"decision\"") && e.contains("both name/alias and banned")));
    }

    fn sample_terms() -> Vec<Term> {
        vec![
            Term {
                name: "terms file".to_string(),
                id: Some("id-terms-file".to_string()),
                definition: String::new(),
                test: String::new(),
                owner: String::new(),
                aliases: vec![],
                banned: vec!["verdict".to_string()],
                retired: false,
                start_line: 1,
                id_line: Some(2),
            },
            Term {
                name: "terms".to_string(),
                id: Some("id-terms".to_string()),
                definition: String::new(),
                test: String::new(),
                owner: String::new(),
                aliases: vec![],
                banned: vec![],
                retired: false,
                start_line: 10,
                id_line: Some(11),
            },
            Term {
                name: "decision".to_string(),
                id: Some("id-decision".to_string()),
                definition: String::new(),
                test: String::new(),
                owner: String::new(),
                aliases: vec![],
                banned: vec![],
                retired: true,
                start_line: 20,
                id_line: Some(21),
            },
        ]
    }

    #[test]
    fn resolve_prefers_the_longer_phrase() {
        let terms = sample_terms();
        let pairs = resolve("read the terms file today", &terms);
        assert_eq!(
            pairs,
            vec![("terms file".to_string(), "id-terms-file".to_string())]
        );
    }

    #[test]
    fn resolve_respects_word_boundaries_no_stemming() {
        let terms = sample_terms();
        let pairs = resolve("the decisions were many", &terms);
        assert!(pairs.is_empty(), "decisions must not match decision: {:?}", pairs);

        let pairs2 = resolve("the decision was final", &terms);
        assert_eq!(
            pairs2,
            vec![("decision".to_string(), "id-decision".to_string())]
        );
    }

    #[test]
    fn resolve_includes_retired_terms_and_exposes_flag() {
        let terms = sample_terms();
        let pairs = resolve("a decision was made", &terms);
        assert!(pairs
            .iter()
            .any(|(w, id)| w == "decision" && id == "id-decision"));
        assert!(terms.iter().find(|t| t.name == "decision").unwrap().retired);
    }

    #[test]
    fn resolve_skips_entries_without_id() {
        let mut terms = sample_terms();
        terms[1].id = None;
        let pairs = resolve("terms terms", &terms);
        assert!(pairs.iter().all(|(_, id)| id != "id-terms"));
    }

    #[test]
    fn banned_hits_finds_a_banned_word_with_its_line() {
        let terms = sample_terms();
        let text = "line one is fine\na verdict was reached\nline three";
        let hits = banned_hits(text, &terms);
        assert_eq!(
            hits,
            vec![(2, "verdict".to_string(), "terms file".to_string())]
        );
    }

    #[test]
    fn banned_hits_respects_word_boundaries() {
        let terms = sample_terms();
        let hits = banned_hits("the overdict is not a match", &terms);
        assert!(hits.is_empty());
    }

    #[test]
    fn fill_missing_ids_inserts_one_line_per_entry_then_nothing() {
        let s = well_formed()
            .replacen("id: 11111111-1111-1111-1111-111111111111\n", "", 1)
            .replacen("id: 22222222-2222-2222-2222-222222222222\n", "", 1);
        let mut tf = parse(&lines_of(&s));
        assert_eq!(tf.terms.len(), 2);
        assert!(tf.terms.iter().all(|t| t.id.is_none()));

        let inserted = fill_missing_ids(&mut tf);
        assert_eq!(inserted, 2);
        assert!(tf.terms.iter().all(|t| t.id.is_some()));
        assert!(tf
            .errors
            .iter()
            .all(|e| !e.contains("missing id")));

        let second = fill_missing_ids(&mut tf);
        assert_eq!(second, 0);
    }

    #[test]
    fn format_and_parse_terms_line_round_trip() {
        let pairs = vec![
            ("decision".to_string(), "11111111-1111-1111-1111-111111111111".to_string()),
            ("question".to_string(), "22222222-2222-2222-2222-222222222222".to_string()),
        ];
        let line = format_terms_line(&pairs).unwrap();
        assert_eq!(
            line,
            "terms: decision=11111111-1111-1111-1111-111111111111, question=22222222-2222-2222-2222-222222222222"
        );
        let parsed = parse_terms_line(&line).unwrap();
        assert_eq!(parsed, pairs);
    }

    #[test]
    fn format_terms_line_is_none_when_empty() {
        assert!(format_terms_line(&[]).is_none());
    }

    #[test]
    fn parse_terms_line_rejects_non_terms_lines() {
        assert!(parse_terms_line("id: abc").is_none());
    }

    #[test]
    fn load_save_and_file_hash_round_trip() {
        let dir = std::env::temp_dir().join(format!("adrdom-terms-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("terms.md");
        fs::write(&path, well_formed()).unwrap();

        let tf = load(&path).unwrap();
        assert!(tf.errors.is_empty(), "unexpected errors: {:?}", tf.errors);
        let h1 = file_hash(&tf);

        save(&tf).unwrap();
        let tf2 = load(&path).unwrap();
        let h2 = file_hash(&tf2);
        assert_eq!(h1, h2);

        fs::remove_dir_all(&dir).ok();
    }
}
