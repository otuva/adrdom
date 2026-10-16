//! The words of a question that pair a term, and the words that pair none:
//! what the answer page underlines in the question and lists beside it.
use super::html::esc;
use crate::record::{self, RecordFile, Section};
use crate::state;
use crate::terms::{self, Term};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

/// The shipped list of common English words (`assets/common_words.txt`,
/// one word per line, lowercase): words that never need a term to explain
/// them, so the "No term" line under a question only names words that
/// might. Not governed data — it ships with the binary.
static COMMON_WORDS: &str = include_str!("../../../assets/common_words.txt");

fn common_words() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        COMMON_WORDS
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect()
    })
}

/// One word on a section's terms line (or its live resolution), paired
/// with the term it names.
pub(super) struct WordPair {
    pub word: String,
    pub term_name: String,
    pub definition: String,
}

/// Every word=term pairing for `sec` (from its terms line if it has one,
/// else computed live over its text) plus the distinct words in that text
/// that pair no term (7.9): not common English, not a uuid or number, not
/// shorter than three letters, and not covered by a pairing. A word pairs
/// to a term only by its name or an explicit alias
/// (521e6211-344a-4b56-a1aa-07be956b3b29, mirrored from `terms::resolve`):
/// an un-aliased plural of a term name is not covered, and still shows up
/// here as "no term".
pub(super) fn section_words(
    rf: &RecordFile,
    sec: &Section,
    terms_list: &[Term],
) -> (Vec<WordPair>, Vec<String>) {
    let text = state::section_text_excluding_terms_line(&rf.lines, sec);
    let pairs: Vec<(String, String)> = match sec.terms_line {
        Some(tl) => terms::parse_terms_line(rf.lines[tl].trim_end()).unwrap_or_default(),
        None => terms::resolve(&text, terms_list),
    };

    let mut words = Vec::new();
    for (w, tid) in &pairs {
        if let Some(t) = terms_list
            .iter()
            .find(|t| t.id.as_deref() == Some(tid.as_str()))
        {
            words.push(WordPair {
                word: w.clone(),
                term_name: t.name.clone(),
                definition: t.definition.clone(),
            });
        }
    }

    let no_entry = no_entry_words(&text, &pairs, terms_list);
    (words, no_entry)
}

/// Removes every dashed uuid (8-4-4-4-12 hex, case-insensitive, on a
/// non-alphanumeric boundary) from `text`, replacing each with a single
/// space. `tokenize` splits a dashed uuid into hex fragments at the
/// hyphens, so without this pass those fragments (not 32 characters long,
/// and not necessarily all-digit) would otherwise show up as "no term"
/// words.
fn strip_uuid_tokens(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < n {
        if let Some(end) = match_uuid_at(&chars, i) {
            let before_ok = i == 0 || !chars[i - 1].is_alphanumeric();
            let after_ok = end == n || !chars[end].is_alphanumeric();
            if before_ok && after_ok {
                out.push(' ');
                i = end;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// If a uuid (8-4-4-4-12 hex groups separated by `-`) starts at `start`,
/// returns the index just past it (`record::is_uuid`, run on the
/// fixed-length candidate slice this shape always is).
fn match_uuid_at(chars: &[char], start: usize) -> Option<usize> {
    let end = start + 36;
    if end > chars.len() {
        return None;
    }
    let candidate: String = chars[start..end].iter().collect();
    record::is_uuid(&candidate).then_some(end)
}

/// A word counts as common (and so never shows up as "No term") when it
/// is in `common_words()` itself, or becomes a listed word once one of
/// these trailing suffixes is stripped: "s" (accepts -> accept, stays ->
/// stay, shapes -> shape, leaves -> leave), "es", "ed" (picked -> pick),
/// "d", or "ing" (staying -> stay, picking -> pick). For "ed" and "ing"
/// only, if stripping the suffix doesn't land on a listed word, trying
/// again with an "e" appended does (shaped/shaping -> "shap" + "e" ->
/// shape). This keeps the shipped word list to base forms instead of
/// spelling out every inflection.
fn is_common_word(w: &str) -> bool {
    let common = common_words();
    if common.contains(w) {
        return true;
    }
    for suf in ["s", "es", "ed", "d", "ing"] {
        if let Some(stripped) = w.strip_suffix(suf) {
            if common.contains(stripped) {
                return true;
            }
            if suf == "ed" || suf == "ing" {
                let with_e = format!("{}e", stripped);
                if common.contains(with_e.as_str()) {
                    return true;
                }
            }
        }
    }
    false
}

/// The words of `text` that pair no term and are not common. A word that is
/// a name or an alias form of a term is checked against the paired terms
/// before the common-word list: paired, it is left out; unpaired, it is
/// named, however common it is in English.
fn no_entry_words(text: &str, pairs: &[(String, String)], terms_list: &[Term]) -> Vec<String> {
    let term_forms: HashSet<String> = terms_list
        .iter()
        .filter(|t| !t.retired)
        .flat_map(|t| std::iter::once(&t.name).chain(t.aliases.iter()))
        .map(|f| f.to_lowercase())
        .collect();
    let mut pair_words: HashSet<String> = HashSet::new();
    for (w, _) in pairs {
        for part in w.to_lowercase().split_whitespace() {
            pair_words.insert(part.to_string());
        }
    }

    let stripped = strip_uuid_tokens(text);
    let mut found: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for line in stripped.split('\n') {
        if crate::tells::starts_with_any(
            line,
            &[
                crate::tells::ID_PREFIX,
                crate::tells::SUPERSEDES_PREFIX,
                crate::tells::UNRELATED_PREFIX,
                crate::tells::BLAME_PREFIX,
            ],
        ) {
            continue;
        }
        for tok in crate::text::word_runs(line) {
            let w = tok.to_lowercase();
            if w.chars().count() < 3 {
                continue;
            }
            if w.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            if term_forms.contains(&w) {
                if !pair_words.contains(&w) {
                    found.insert(w);
                }
                continue;
            }
            if is_common_word(&w) || pair_words.contains(&w) {
                continue;
            }
            found.insert(w);
        }
    }
    found.into_iter().collect()
}

#[cfg(test)]
mod word_tests {
    use super::is_common_word;

    #[test]
    fn is_common_word_covers_inflected_common_words() {
        assert!(is_common_word("accepts"), "accepts -> accept");
        assert!(is_common_word("picked"), "picked -> pick");
        assert!(is_common_word("stays"), "stays -> stay");
        assert!(is_common_word("shapes"), "shapes -> shape");
        assert!(is_common_word("beside"), "beside is a plain base word");
    }

    #[test]
    fn a_term_word_is_checked_against_the_paired_terms_before_the_common_words() {
        let term = |name: &str, aliases: &[&str]| crate::terms::Term {
            name: name.to_string(),
            id: Some(format!("id-{}", name)),
            definition: "d".to_string(),
            aliases: aliases.iter().map(|a| a.to_string()).collect(),
            retired: false,
            start_line: 0,
            id_line: None,
            terms_line: None,
            terms_line_idx: None,
        };
        let terms = vec![term("question", &["questions"])];
        // Unpaired, a term word is named though English uses it every day.
        assert_eq!(
            super::no_entry_words("A question here", &[], &terms),
            vec!["question".to_string()]
        );
        assert_eq!(
            super::no_entry_words("Many questions here", &[], &terms),
            vec!["questions".to_string()]
        );
        // Paired, it is left out.
        let pairs = vec![("question".to_string(), "id-question".to_string())];
        assert!(super::no_entry_words("A question here", &pairs, &terms).is_empty());
        // A common word that is no term stays common.
        assert!(super::no_entry_words("A thing here", &[], &terms).is_empty());
    }

    #[test]
    fn is_common_word_does_not_swallow_real_no_term_words() {
        assert!(!is_common_word("principle"));
        assert!(!is_common_word("constraint"));
    }
}

/// Wraps every whole-word (case-insensitive) occurrence of a paired word
/// in `text` with `<span class="t" title="<term name>: <definition>">
/// ...</span>`, preserving the text's original casing inside the span. A
/// multi-word pairing (e.g. "topic file") wins over a shorter one that
/// starts the same way (e.g. "topic") when both match at a position.
/// Every piece of `text` is HTML-escaped; the `<span>` wrapper itself
/// never is.
pub(super) fn mark_terms(text: &str, words: &[WordPair]) -> String {
    if words.is_empty() {
        return esc(text);
    }

    let mut by_phrase: HashMap<String, &WordPair> = HashMap::new();
    let mut max_len = 1usize;
    for w in words {
        let key = w.word.to_lowercase();
        max_len = max_len.max(key.split_whitespace().count().max(1));
        by_phrase.insert(key, w);
    }

    enum Piece {
        Word(String),
        Other(String),
    }

    let mut pieces: Vec<Piece> = Vec::new();
    let mut cur = String::new();
    let mut cur_is_word = false;
    let mut started = false;
    for c in text.chars() {
        let is_word = crate::text::is_word_char(c);
        if started && is_word != cur_is_word {
            let taken = std::mem::take(&mut cur);
            pieces.push(if cur_is_word {
                Piece::Word(taken)
            } else {
                Piece::Other(taken)
            });
        }
        cur.push(c);
        cur_is_word = is_word;
        started = true;
    }
    if !cur.is_empty() {
        pieces.push(if cur_is_word {
            Piece::Word(cur)
        } else {
            Piece::Other(cur)
        });
    }

    let mut out = String::new();
    let mut i = 0;
    while i < pieces.len() {
        let is_word_piece = matches!(pieces[i], Piece::Word(_));
        if !is_word_piece {
            if let Piece::Other(o) = &pieces[i] {
                out.push_str(&esc(o));
            }
            i += 1;
            continue;
        }

        let mut matched: Option<(usize, &WordPair)> = None;
        let mut len = max_len;
        while len >= 1 {
            let mut ok = true;
            let mut phrase_words: Vec<&str> = Vec::new();
            for k in 0..len {
                let idx = i + 2 * k;
                match pieces.get(idx) {
                    Some(Piece::Word(w)) => phrase_words.push(w.as_str()),
                    _ => {
                        ok = false;
                        break;
                    }
                }
                if k + 1 < len {
                    match pieces.get(idx + 1) {
                        Some(Piece::Other(o)) if o == " " => {}
                        _ => {
                            ok = false;
                            break;
                        }
                    }
                }
            }
            if ok {
                let phrase = phrase_words.join(" ").to_lowercase();
                if let Some(wp) = by_phrase.get(&phrase) {
                    matched = Some((len, wp));
                    break;
                }
            }
            len -= 1;
        }

        if let Some((len, wp)) = matched {
            let end = i + 2 * (len - 1) + 1;
            let mut inner = String::new();
            for piece in &pieces[i..end] {
                match piece {
                    Piece::Word(w) => inner.push_str(&esc(w)),
                    Piece::Other(o) => inner.push_str(&esc(o)),
                }
            }
            out.push_str(&format!(
                "<span class=\"t\" title=\"{}: {}\">{}</span>",
                esc(&wp.term_name),
                esc(&wp.definition),
                inner
            ));
            i = end;
        } else {
            if let Piece::Word(w) = &pieces[i] {
                out.push_str(&esc(w));
            }
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{mark_terms, WordPair};

    fn wp(word: &str, term_name: &str, definition: &str) -> WordPair {
        WordPair {
            word: word.to_string(),
            term_name: term_name.to_string(),
            definition: definition.to_string(),
        }
    }

    #[test]
    fn mark_terms_leaves_a_plain_word_untouched() {
        assert_eq!(mark_terms("hello world", &[]), "hello world");
    }

    #[test]
    fn mark_terms_wraps_a_paired_word() {
        let words = [wp("decision", "decision", "The choice.")];
        let out = mark_terms("a decision was made", &words);
        assert!(
            out.contains(r#"<span class="t" title="decision: The choice.">decision</span>"#),
            "{}",
            out
        );
    }

    #[test]
    fn mark_terms_preserves_case() {
        let words = [wp("decision", "decision", "The choice.")];
        let out = mark_terms("Decision", &words);
        assert!(
            out.contains(r#"<span class="t" title="decision: The choice.">Decision</span>"#),
            "{}",
            out
        );
    }

    #[test]
    fn mark_terms_prefers_the_longer_pairing() {
        let words = [
            wp("topic", "topic", "One subject."),
            wp("topic file", "topic file", "A markdown file."),
        ];
        let out = mark_terms("read the topic file today", &words);
        assert!(
            out.contains(
                r#"<span class="t" title="topic file: A markdown file.">topic file</span>"#
            ),
            "{}",
            out
        );
        assert!(!out.contains("title=\"topic:"), "{}", out);
    }

    #[test]
    fn mark_terms_escapes_html_in_text() {
        assert_eq!(mark_terms("A & B", &[]), "A &amp; B");
    }
}
