//! Resolving the words of a text to the terms they name.

use super::{Term, TermsFile};
use std::collections::{BTreeSet, HashMap};

fn is_boundary_ok(chars: &[char], start: usize, len: usize) -> bool {
    let before_ok = start == 0 || !crate::text::is_word_char(chars[start - 1]);
    let after_ok = start + len == chars.len() || !crate::text::is_word_char(chars[start + len]);
    before_ok && after_ok
}

/// Every name and alias in `terms`, lowercased, mapped to the id of the term
/// that owns it. A word pairs to a term only when it equals the term's name
/// or one of its aliases exactly -- no implicit plural or other stemming
/// (per "Does a term word keep its plain sense in the records?"): every form
/// that should pair is written as an explicit alias. Terms without an id,
/// and retired terms, are skipped (per "Does a retired term hold its
/// words?"): a lookup never lands on a retired term, so a new pairing sees
/// current terms only.
///
/// A single lookup against this map answers whether a whole word/phrase
/// still resolves to a given id exactly as `resolve(word, terms)` would
/// (a stored pairing is always the whole matched text, so only a
/// full-string candidate can produce it); built once and reused across
/// many words instead of rebuilding `resolve`'s phrase list per word.
pub fn phrase_lookup(terms: &[Term]) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();
    for t in terms {
        if t.retired {
            continue;
        }
        let Some(id) = &t.id else { continue };
        let mut names = vec![t.name.clone()];
        names.extend(t.aliases.iter().cloned());
        for n in names {
            let nl = n.trim().to_lowercase();
            if nl.is_empty() {
                continue;
            }
            map.entry(nl).or_insert_with(|| id.clone());
        }
    }
    map
}

/// True when `text` (trimmed, case-insensitive) equals the name or an alias
/// of a retired term: the same normalisation `phrase_lookup` applies.
pub fn is_retired_phrase(terms: &[Term], text: &str) -> bool {
    let lower = text.to_lowercase();
    terms.iter().any(|t| {
        t.retired
            && std::iter::once(&t.name)
                .chain(t.aliases.iter())
                .any(|n| n.trim().to_lowercase() == lower)
    })
}

/// Whole-word/phrase, case-insensitive, longest-phrase-first, non-overlapping
/// resolution of every term name/alias in `text`. Retired terms are never
/// matched (per "Does a retired term hold its words?"): every new pairing sees current terms only, so a word
/// that only a retired term owns resolves to nothing here (a current term
/// is free to take that same name or alias). Terms without an id are
/// skipped. Returned pairs are unique and sorted by matched word.
///
/// A word matches a name or alias only when it equals that name/alias
/// exactly (per "Does a term word keep its plain sense in the records?"):
/// there is no implicit plural or other stemming, so a plural or other
/// variant form pairs only when it is itself written as an alias.
pub fn resolve(text: &str, terms: &[Term]) -> Vec<(String, String)> {
    let mut phrases: Vec<(Vec<char>, String)> = Vec::new();
    for t in terms {
        if t.retired {
            continue;
        }
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

/// Resolves term `idx`'s definition text against every current term in
/// `tf` (including itself, so a self-pairing can be detected; a retired
/// `tf.terms[idx]` never matches, since `resolve` skips retired targets).
///
/// Used by `cycles` below and by approve's circular-definition check.
pub fn definition_pairs(tf: &TermsFile, idx: usize) -> Vec<(String, String)> {
    resolve(&tf.terms[idx].definition, &tf.terms)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_terms() -> Vec<Term> {
        vec![
            Term {
                name: "terms file".to_string(),
                id: Some("id-terms-file".to_string()),
                definition: String::new(),
                aliases: vec![],
                retired: false,
                start_line: 1,
                id_line: Some(2),
                terms_line: None,
                terms_line_idx: None,
            },
            Term {
                name: "terms".to_string(),
                id: Some("id-terms".to_string()),
                definition: String::new(),
                aliases: vec![],
                retired: false,
                start_line: 10,
                id_line: Some(11),
                terms_line: None,
                terms_line_idx: None,
            },
            Term {
                name: "decision".to_string(),
                id: Some("id-decision".to_string()),
                definition: String::new(),
                aliases: vec![],
                retired: false,
                start_line: 20,
                id_line: Some(21),
                terms_line: None,
                terms_line_idx: None,
            },
            Term {
                name: "verdict".to_string(),
                id: Some("id-verdict".to_string()),
                definition: String::new(),
                aliases: vec![],
                retired: true,
                start_line: 30,
                id_line: Some(31),
                terms_line: None,
                terms_line_idx: None,
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

    // adr: 521e6211-344a-4b56-a1aa-07be956b3b29
    //
    // "a term word always means the term in the records; its plain sense
    // leaves them, forms that mean the term are aliases, and plain use of
    // the same form is caught at review": there is no implicit "s"/"es"
    // plural match any more, so a plural that is not listed as an alias is
    // a plain word, and the exact same form pairs once it is added as an
    // alias.
    #[test]
    fn a_plural_form_pairs_only_when_it_is_an_explicit_alias() {
        let terms = sample_terms();

        // "decisions" is not an alias of "decision" here: a plain word,
        // caught at review rather than paired.
        let pairs = resolve("the decisions were many", &terms);
        assert!(
            pairs.iter().all(|(_, id)| id != "id-decision"),
            "{:?}",
            pairs
        );

        // Not a plural of anything registered either: no arbitrary stemming.
        let pairs2 = resolve("a decisional matter", &terms);
        assert!(pairs2.iter().all(|(_, id)| id != "id-decision"));

        // Exact singular still matches.
        let pairs3 = resolve("the decision was final", &terms);
        assert_eq!(
            pairs3,
            vec![("decision".to_string(), "id-decision".to_string())]
        );

        // The same plural form pairs once it is written as an explicit
        // alias.
        let mut terms_with_alias = terms.clone();
        terms_with_alias
            .iter_mut()
            .find(|t| t.name == "decision")
            .unwrap()
            .aliases
            .push("decisions".to_string());
        let pairs4 = resolve("the decisions were many", &terms_with_alias);
        assert_eq!(
            pairs4,
            vec![("decisions".to_string(), "id-decision".to_string())]
        );
    }

    #[test]
    fn resolve_plural_does_not_shadow_a_longer_phrase() {
        let terms = vec![
            Term {
                name: "cat".to_string(),
                id: Some("id-cat".to_string()),
                definition: String::new(),
                aliases: vec![],
                retired: false,
                start_line: 1,
                id_line: Some(2),
                terms_line: None,
                terms_line_idx: None,
            },
            Term {
                name: "cats meow".to_string(),
                id: Some("id-cats-meow".to_string()),
                definition: String::new(),
                aliases: vec![],
                retired: false,
                start_line: 10,
                id_line: Some(11),
                terms_line: None,
                terms_line_idx: None,
            },
        ];
        let pairs = resolve("the cats meow loudly", &terms);
        assert_eq!(
            pairs,
            vec![("cats meow".to_string(), "id-cats-meow".to_string())]
        );
    }

    // adr: 521e6211-344a-4b56-a1aa-07be956b3b29
    #[test]
    fn resolve_matches_irregular_plural_via_alias() {
        let terms = vec![Term {
            name: "mouse".to_string(),
            id: Some("id-mouse".to_string()),
            definition: String::new(),
            aliases: vec!["mice".to_string()],
            retired: false,
            start_line: 1,
            id_line: Some(2),
            terms_line: None,
            terms_line_idx: None,
        }];
        let pairs = resolve("the mice ran", &terms);
        assert_eq!(pairs, vec![("mice".to_string(), "id-mouse".to_string())]);
        // No automatic plural is formed from "mouse" at all; "mice" pairs
        // only because it is listed as an explicit alias.
        let pairs2 = resolve("a mouse ran", &terms);
        assert_eq!(pairs2, vec![("mouse".to_string(), "id-mouse".to_string())]);
    }

    #[test]
    fn resolve_excludes_retired_terms() {
        // "verdict" is retired in `sample_terms`; a word that only a
        // retired term owns pairs nothing at all (per "Does a retired term hold its words?").
        let terms = sample_terms();
        assert!(terms.iter().find(|t| t.name == "verdict").unwrap().retired);
        let pairs = resolve("the verdict was reached", &terms);
        assert!(
            pairs.iter().all(|(_, id)| id != "id-verdict"),
            "{:?}",
            pairs
        );
        // A current term is unaffected.
        let pairs2 = resolve("a decision was made", &terms);
        assert!(pairs2
            .iter()
            .any(|(w, id)| w == "decision" && id == "id-decision"));
    }

    #[test]
    fn phrase_lookup_excludes_retired_terms() {
        let terms = sample_terms();
        let lookup = phrase_lookup(&terms);
        assert!(!lookup.values().any(|id| id == "id-verdict"));
        assert_eq!(
            lookup.get("decision").map(String::as_str),
            Some("id-decision")
        );
    }

    // `resolve_word` (the per-call, per-entry resolver) was folded into
    // `depends::resolve_dep`, which now takes a `phrase_lookup` built once
    // per command run instead of rebuilding it on every `depends:` entry;
    // these three cases move to `phrase_lookup` directly, the map that
    // backed it and still backs `resolve_dep`.

    #[test]
    fn phrase_lookup_word_rejects_a_retired_term() {
        let terms = sample_terms();
        let lookup = phrase_lookup(&terms);
        // "verdict" is retired in `sample_terms()`.
        assert_eq!(lookup.get("verdict"), None);
    }

    #[test]
    fn phrase_lookup_word_resolves_a_current_term_by_name_or_phrase() {
        let terms = sample_terms();
        let lookup = phrase_lookup(&terms);
        assert_eq!(lookup.get("terms"), Some(&"id-terms".to_string()));
        assert_eq!(lookup.get("terms file"), Some(&"id-terms-file".to_string()));
    }

    #[test]
    fn phrase_lookup_word_returns_none_for_an_unknown_word() {
        let terms = sample_terms();
        let lookup = phrase_lookup(&terms);
        assert_eq!(lookup.get("nosuchword"), None);
    }

    #[test]
    fn resolve_skips_entries_without_id() {
        let mut terms = sample_terms();
        terms[1].id = None;
        let pairs = resolve("terms terms", &terms);
        assert!(pairs.iter().all(|(_, id)| id != "id-terms"));
    }
}
