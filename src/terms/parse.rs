//! Parsing of the terms file text into terms, and the sentence count of a definition.

use super::{Term, TermsFile};
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

/// `"key:value"` or `"key: value"`, trimmed: unlike `record::kv`, the space
/// after the colon is optional, since a real `adrdom/terms.md` field with
/// an empty value is written with no trailing space at all (e.g. an
/// aliases-less term's bare `aliases:` line) -- `record::kv` would treat
/// that as not matching the key at all.
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
        if name.trim().is_empty() {
            errors.push(format!("line {}: empty name", start + 1));
        }

        let mut id: Option<String> = None;
        let mut id_line: Option<usize> = None;
        let mut definition: Option<String> = None;
        let mut aliases: Option<String> = None;
        let mut terms_line: Option<String> = None;
        let mut terms_line_idx: Option<usize> = None;
        let mut retired = false;
        let mut retired_seen = false;

        for (j, l) in lines.iter().enumerate().take(end).skip(start + 1) {
            if l.trim().is_empty() {
                continue;
            }
            if l.trim_end() == "# Terms" {
                // Already reported as a duplicate h1 above.
                continue;
            }
            if kv(l, "id").is_some() {
                if id.is_none() {
                    id = kv(l, "id");
                    id_line = Some(j);
                } else {
                    errors.push(format!("line {}: duplicate field \"id\"", j + 1));
                }
            } else if kv(l, "definition").is_some() {
                if definition.is_none() {
                    definition = kv(l, "definition");
                } else {
                    errors.push(format!("line {}: duplicate field \"definition\"", j + 1));
                }
            } else if kv(l, "aliases").is_some() {
                if aliases.is_none() {
                    aliases = kv(l, "aliases");
                } else {
                    errors.push(format!("line {}: duplicate field \"aliases\"", j + 1));
                }
            } else if kv(l, "terms").is_some() {
                if terms_line.is_none() {
                    terms_line = Some(l.trim_end().to_string());
                    terms_line_idx = Some(j);
                } else {
                    errors.push(format!("line {}: duplicate field \"terms\"", j + 1));
                }
            } else if let Some(v) = kv(l, "retired") {
                if retired_seen {
                    errors.push(format!("line {}: duplicate field \"retired\"", j + 1));
                } else {
                    retired_seen = true;
                    if v == "yes" {
                        retired = true;
                    } else {
                        errors.push(format!("line {}: unknown line", j + 1));
                    }
                }
            } else {
                errors.push(format!("line {}: unknown line", j + 1));
            }
        }

        // An entry with no id is no shape fault: only approve writes ids,
        // so such a file is one that waits for its approval
        // (`missing_ids`), the owner's move alone.
        if definition.is_none() {
            errors.push(format!("line {}: missing field \"definition\"", start + 1));
        }
        if aliases.is_none() {
            errors.push(format!("line {}: missing field \"aliases\"", start + 1));
        }

        terms.push(Term {
            name,
            id,
            definition: definition.unwrap_or_default(),
            aliases: split_csv(aliases.as_deref().unwrap_or("")),
            retired,
            start_line: start + 1,
            id_line: id_line.map(|x| x + 1),
            terms_line,
            terms_line_idx: terms_line_idx.map(|x| x + 1),
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

/// A retired entry's name and aliases are ignored here, so a current term
/// may take a retired term's word (per "Does a retired term hold its words?"); two current entries sharing a
/// word still refuse.
fn duplicate_word_errors(terms: &[Term]) -> Vec<String> {
    let mut occ: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, t) in terms.iter().enumerate() {
        if t.retired {
            continue;
        }
        let mut namealias = vec![t.name.clone()];
        namealias.extend(t.aliases.iter().cloned());
        for w in namealias {
            let wl = w.trim().to_lowercase();
            if wl.is_empty() {
                continue;
            }
            occ.entry(wl).or_default().push(idx);
        }
    }

    let mut errors = Vec::new();
    let mut reported: BTreeSet<(String, usize, usize)> = BTreeSet::new();
    let mut words: Vec<&String> = occ.keys().collect();
    words.sort();
    for word in words {
        let mut entries = occ[word].clone();
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
        }
    }
    errors
}

/// Sentences in `text` (fcc1c1ec-d8e8-428e-9731-9817cdeb8e76): a `.`, `?`
/// or `!` ends a sentence only when it is followed by the end of the text,
/// or by whitespace and then an uppercase letter, a digit, or an opening
/// quote, backtick, or bracket -- so an abbreviation like "e.g." or "i.e."
/// mid-sentence, whose next word starts lowercase, is not counted as a
/// sentence end.
pub fn sentence_count(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut count = 0;
    for (i, &c) in chars.iter().enumerate() {
        if c != '.' && c != '?' && c != '!' {
            continue;
        }
        if i + 1 == chars.len() {
            count += 1;
            continue;
        }
        if !chars[i + 1].is_whitespace() {
            continue;
        }
        let next = chars[(i + 1)..].iter().find(|c| !c.is_whitespace());
        match next {
            None => count += 1,
            Some(&n) => {
                if n.is_uppercase()
                    || n.is_ascii_digit()
                    || matches!(n, '"' | '\'' | '`' | '(' | '[' | '{')
                {
                    count += 1;
                }
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terms::tests::{lines_of, well_formed};

    // adr: 7c306235-6243-4d79-b9aa-08ed19662779
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
        assert!(!tf.terms[0].retired);
        assert!(tf.terms[1].aliases.is_empty());
    }

    #[test]
    fn retired_line_is_recognized() {
        let mut s = well_formed();
        s = s.replacen("aliases: choice\n", "aliases: choice\nretired: yes\n", 1);
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
        let s = well_formed().replacen("definition: A choice answered by the owner.\n", "", 1);
        let tf = parse(&lines_of(&s));
        assert!(tf
            .errors
            .iter()
            .any(|e| e.contains("missing field \"definition\"")));
    }

    #[test]
    fn unknown_line_is_a_shape_error() {
        let s = well_formed().replacen("aliases: choice\n", "aliases: choice\nnotes: extra\n", 1);
        let tf = parse(&lines_of(&s));
        assert!(tf.errors.iter().any(|e| e.contains("unknown line")));
    }

    // adr: c6da863d-5085-402f-815a-9bb832b4df7e
    // A term entry carries no `banned` field: the ban list is the only
    // place a term's banned words are kept.
    #[test]
    fn removed_fields_are_unknown_lines() {
        for field in ["test", "owner", "banned"] {
            let s = well_formed().replacen(
                "aliases: choice\n",
                &format!("aliases: choice\n{}: x\n", field),
                1,
            );
            let tf = parse(&lines_of(&s));
            assert!(
                tf.errors.iter().any(|e| e.contains("unknown line")),
                "field \"{}\" not reported as unknown line: {:?}",
                field,
                tf.errors
            );
        }
    }

    #[test]
    fn duplicate_field_is_a_shape_error() {
        let s = well_formed().replacen(
            "definition: A choice answered by the owner.\n",
            "definition: A choice answered by the owner.\ndefinition: Something else.\n",
            1,
        );
        let tf = parse(&lines_of(&s));
        assert!(tf
            .errors
            .iter()
            .any(|e| e.contains("duplicate field \"definition\"")));
    }

    #[test]
    fn missing_id_is_an_entry_awaiting_approval_not_a_shape_error() {
        let s = well_formed().replacen("id: 11111111-1111-1111-1111-111111111111\n", "", 1);
        let tf = parse(&lines_of(&s));
        assert!(tf.errors.is_empty(), "{:?}", tf.errors);
        assert_eq!(tf.missing_ids().len(), 1);
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
    fn duplicate_word_across_two_entries_is_an_error() {
        let s = well_formed().replacen("aliases: choice\n", "aliases: question\n", 1);
        let tf = parse(&lines_of(&s));
        assert!(tf.errors.iter().any(|e| e.contains("word \"question\"")
            && e.contains("decision")
            && e.contains("question")));
    }

    #[test]
    fn empty_name_is_a_shape_error() {
        let s = well_formed().replacen("## decision\n", "## \n", 1);
        let tf = parse(&lines_of(&s));
        assert!(tf.errors.iter().any(|e| e.contains("empty name")));
    }

    // adr: fcc1c1ec-d8e8-428e-9731-9817cdeb8e76
    #[test]
    fn sentence_count_counts_terminators_followed_by_whitespace_or_end() {
        assert_eq!(sentence_count("What the thing is."), 1);
        assert_eq!(sentence_count("What the thing is. Never what it does."), 2);
        assert_eq!(
            sentence_count("What the thing is. Never what it does. Or how it is used."),
            3
        );
        // A `.` not followed by whitespace or end (e.g. inside a word) is
        // not a terminator.
        assert_eq!(sentence_count("Version 1.5 of the thing."), 1);
    }

    // adr: fcc1c1ec-d8e8-428e-9731-9817cdeb8e76
    //
    // A `.` inside an abbreviation like "e.g." or "i.e." is followed by
    // whitespace, but the word after that whitespace starts lowercase, so
    // it is not a sentence end; only the terminator at the true end of the
    // text counts.
    #[test]
    fn an_abbreviation_dot_mid_sentence_does_not_end_a_sentence() {
        assert_eq!(sentence_count("Use e.g. the shared helper for this."), 1);
        assert_eq!(sentence_count("Ship i.e. one path only."), 1);
        assert_eq!(sentence_count("Ship it, etc. and stop there."), 1);
        assert_eq!(
            sentence_count("One sentence here. Another sentence here."),
            2
        );
    }

    #[test]
    fn duplicate_word_errors_ignores_a_retired_entry() {
        // A retired entry and a current entry sharing the same word: no
        // conflict, since the retired one is not counted (per "Does a retired term hold its words?").
        let terms = vec![
            Term {
                name: "decision".to_string(),
                id: Some("id-old".to_string()),
                definition: String::new(),
                aliases: vec![],
                retired: true,
                start_line: 1,
                id_line: Some(2),
                terms_line: None,
                terms_line_idx: None,
            },
            Term {
                name: "decision".to_string(),
                id: Some("id-new".to_string()),
                definition: String::new(),
                aliases: vec![],
                retired: false,
                start_line: 10,
                id_line: Some(11),
                terms_line: None,
                terms_line_idx: None,
            },
        ];
        let errors = duplicate_word_errors(&terms);
        assert!(errors.is_empty(), "{:?}", errors);
    }
}
