//! The `/next` route: picks the next question to answer and renders it,
//! marking paired term words in its text.
use super::html::{banner_html, esc, neighbours_html, page};
use super::percent_encode;
use crate::view::{self, FileView, QuestionStatus, QuestionView, Snapshot};
use std::collections::HashMap;

pub(super) fn render_next(
    snap: &Snapshot,
    after: Option<&String>,
    show: Option<&String>,
    m: Option<&String>,
    token: &str,
) -> String {
    let mut body = String::new();
    body.push_str(&banner_html(m));

    let all = full_question_list(snap);
    let picked = pick_next(&all, after.map(String::as_str), show.map(String::as_str));

    let Some((file, q)) = picked else {
        // An asked question `answer` would refuse waits on another one:
        // nothing is wrong, but "every question is answered" would be false.
        let waiting = all.iter().any(|(_, q)| q.state == QuestionStatus::Asked);
        let title = if waiting { "No question can be answered yet" } else { "Every question is answered" };
        body.push_str(&format!("<header class=\"lead\">\n<h1>{}</h1>\n", title));
        body.push_str("<p><a href=\"/\">Back to the overview</a></p>\n</header>\n");
        return page("Answer", "/next", &body, true);
    };

    let id = q.id.clone().unwrap_or_default();
    // "Question p of N open" counts what the owner can act on now: the
    // questions `answer` takes, the same ones the page offers.
    let open_ids: Vec<&str> = all
        .iter()
        .filter(|(_, x)| x.answerable)
        .filter_map(|(_, x)| x.id.as_deref())
        .collect();
    let position = open_ids.iter().position(|x| *x == id).map(|p| p + 1);

    if show.is_none() && after.map(String::as_str) == Some(id.as_str()) {
        body.push_str("<p class=\"waits\">This is the only question left to answer.</p>\n");
    }
    body.push_str("<article class=\"focus\">\n");
    body.push_str("<p class=\"where\">");
    let tier = q
        .tier
        .map(|t| t.to_string())
        .unwrap_or_else(|| "?".to_string());
    body.push_str(&format!(
        "{}, tier {} @{}",
        esc(&file.topic),
        esc(&tier),
        esc(&q.version)
    ));
    if let Some(p) = position {
        body.push_str(&format!(". Question {} of {} open", p, open_ids.len()));
    }
    body.push_str(".</p>\n");
    body.push_str(&format!("<h1>{}</h1>\n", mark_terms(&q.heading, &q.words)));
    if !q.depends.is_empty() {
        body.push_str(&format!(
            "<p class=\"stands-on\">Stands on: {}</p>\n",
            esc(&q.depends.join(", "))
        ));
    }
    body.push_str(&neighbours_html(&q.neighbours));

    if q.problem.is_some() || q.standard.is_some() {
        body.push_str("<dl class=\"framing\">\n");
        if let Some(p) = &q.problem {
            body.push_str(&format!(
                "<dt>Problem</dt><dd>{}</dd>\n",
                mark_terms(p, &q.words)
            ));
        }
        if let Some(s) = &q.standard {
            body.push_str(&format!(
                "<dt>Standard</dt><dd>{}</dd>\n",
                mark_terms(s, &q.words)
            ));
        }
        body.push_str("</dl>\n");
    }

    body.push_str("<aside class=\"tq\">\n<h2>Terms in this question</h2>\n");
    if q.words.is_empty() {
        body.push_str("<p class=\"meta\">No words pair a term.</p>\n");
    } else {
        // Group by term name, first-seen order (q.words is already sorted
        // by word, so the first occurrence of a term name is stable).
        let mut order: Vec<&str> = Vec::new();
        let mut groups: HashMap<&str, (&str, Vec<&str>)> = HashMap::new();
        for w in &q.words {
            let entry = groups.entry(w.term_name.as_str()).or_insert_with(|| {
                order.push(w.term_name.as_str());
                (w.definition.as_str(), Vec::new())
            });
            if w.word != w.term_name {
                entry.1.push(w.word.as_str());
            }
        }
        body.push_str("<dl>\n");
        for name in &order {
            let (def, forms) = &groups[name];
            body.push_str(&format!("<dt>{}", esc(name)));
            if !forms.is_empty() {
                body.push_str(&format!(
                    "<span class=\"forms\">{}</span>",
                    esc(&forms.join(", "))
                ));
            }
            body.push_str(&format!("</dt><dd>{}</dd>\n", esc(def)));
        }
        body.push_str("</dl>\n");
    }
    if !q.no_entry.is_empty() {
        body.push_str(&format!(
            "<p class=\"none\"><b>No term:</b> {}</p>\n",
            esc(&q.no_entry.join(", "))
        ));
    }
    body.push_str("</aside>\n");

    // An asked question `answer` would refuse (reached by a link or a
    // `show` address): what it waits on, and no answer form.
    if !q.answerable {
        let why = if q.waits_on.is_empty() {
            "was edited since it was asked; the agent asks it again".to_string()
        } else {
            let names: Vec<String> = q.waits_on.iter().map(|h| format!("<q>{}</q>", esc(h))).collect();
            format!("waits for {}", names.join(", "))
        };
        body.push_str(&format!(
            "<p class=\"waits\">Not yet: this question {}.</p>\n<p><a class=\"skip\" href=\"/next?after={}\">Skip this one</a></p>\n</article>\n",
            why,
            percent_encode(&id)
        ));
        return page("Answer", "/next", &body, true);
    }

    body.push_str(&format!(
        "<form method=\"post\" action=\"/answer\" id=\"answer-form\">\n<input type=\"hidden\" name=\"id\" value=\"{}\">\n<input type=\"hidden\" name=\"v\" value=\"{}\">\n<input type=\"hidden\" name=\"t\" value=\"{}\">\n",
        esc(&id),
        esc(&q.version),
        esc(token)
    ));
    body.push_str("<div class=\"options\" role=\"radiogroup\">\n");
    for (i, o) in q.options.iter().enumerate() {
        let input_id = format!("opt-{}", o.letter);
        body.push_str(&format!(
            "<label class=\"option\" for=\"{iid}\"><input type=\"radio\" name=\"answer\" id=\"{iid}\" value=\"{letter}\"{checked}><span class=\"key\">{letter}</span><span class=\"text\"><span class=\"sentence\">{sentence}</span>",
            iid = input_id,
            letter = o.letter,
            checked = if i == 0 { " checked" } else { "" },
            sentence = mark_terms(&o.sentence, &q.words),
        ));
        if o.why.is_some() || o.costs.is_some() {
            body.push_str("<dl class=\"rationale\">");
            if let Some(why) = &o.why {
                body.push_str(&format!(
                    "<dt>Why</dt><dd>{}</dd>",
                    mark_terms(why, &q.words)
                ));
            }
            if let Some(costs) = &o.costs {
                body.push_str(&format!(
                    "<dt>Costs</dt><dd>{}</dd>",
                    mark_terms(costs, &q.words)
                ));
            }
            body.push_str("</dl>");
        }
        body.push_str("</span></label>\n");
    }
    body.push_str("</div>\n");
    let first = q.options.first().map(|o| o.letter).unwrap_or('A');
    // The Refuse button shares the `answer` form field with the letter
    // radios, submitting `answer=refuse` when it (not a radio) is the
    // control that submits the form (per "Does the owner answer a question
    // with a refusal?"). `actions::do_answer` takes this the same way it
    // takes a letter, via `do_answer`/`do_refuse`'s own handling of the
    // value "refuse".
    body.push_str(&format!(
        "<p class=\"act\"><button type=\"submit\" class=\"answer\" id=\"answer-button\">Answer <span id=\"answer-letter\">{}</span></button> <button type=\"submit\" name=\"answer\" value=\"refuse\" class=\"refuse\" id=\"refuse-button\">Refuse</button> <a class=\"skip\" href=\"/next?after={}\">Skip this one</a></p>\n",
        first,
        percent_encode(&id)
    ));
    body.push_str("</form>\n");
    body.push_str("<p class=\"key-hint\">Press a letter to choose, <kbd>Enter</kbd> to answer, <kbd>S</kbd> to skip.</p>\n");
    body.push_str("</article>\n");

    body.push_str(&format!(
        r#"<script>
(function() {{
  var form = document.getElementById('answer-form');
  var letter = document.getElementById('answer-letter');
  function sync() {{
    var chosen = form.querySelector('input[name=answer]:checked');
    if (chosen && letter) {{ letter.textContent = chosen.value; }}
  }}
  form.addEventListener('change', sync);
  sync();
  document.addEventListener('keydown', function(e) {{
    if (e.altKey || e.ctrlKey || e.metaKey) return;
    var tag = (document.activeElement && document.activeElement.tagName) || '';
    if (tag === 'INPUT' || tag === 'TEXTAREA') {{
      if (e.key === 'Enter') {{ form.submit(); }}
      return;
    }}
    var key = e.key.toUpperCase();
    if (key.length === 1 && key >= 'A' && key <= 'E') {{
      var radio = document.getElementById('opt-' + key);
      if (radio) {{ radio.checked = true; sync(); }}
    }} else if (key === 'ENTER') {{
      form.submit();
    }} else if (key === 'S') {{
      window.location.href = '/next?after={id}';
    }}
  }});
}})();
</script>
"#,
        id = percent_encode(&id)
    ));

    page("Answer", "/next", &body, true)
}

fn full_question_list(snap: &Snapshot) -> Vec<(&FileView, &QuestionView)> {
    let mut out = Vec::new();
    for f in &snap.files {
        for q in &f.questions {
            out.push((f, q));
        }
    }
    out
}

fn pick_next<'a>(
    all: &[(&'a FileView, &'a QuestionView)],
    after: Option<&str>,
    show: Option<&str>,
) -> Option<(&'a FileView, &'a QuestionView)> {
    if let Some(show_id) = show {
        if let Some(found) = all.iter().find(|(_, q)| q.id.as_deref() == Some(show_id)) {
            return Some(*found);
        }
    }
    if let Some(after_id) = after {
        if let Some(pos) = all
            .iter()
            .position(|(_, q)| q.id.as_deref() == Some(after_id))
        {
            // Forward in file order first, then wrap: an answer can unlock
            // a question above it.
            // The skipped question comes last: when it is the only one
            // left, it is offered again.
            return all[pos + 1..]
                .iter()
                .chain(all[..=pos].iter())
                .find(|(_, q)| q.answerable)
                .copied();
        }
    }
    all.iter().find(|(_, q)| q.answerable).copied()
}

/// Wraps every whole-word (case-insensitive) occurrence of a paired word
/// in `text` with `<span class="t" title="<term name>: <definition>">
/// ...</span>`, preserving the text's original casing inside the span. A
/// multi-word pairing (e.g. "topic file") wins over a shorter one that
/// starts the same way (e.g. "topic") when both match at a position.
/// Every piece of `text` is HTML-escaped; the `<span>` wrapper itself
/// never is.
fn mark_terms(text: &str, words: &[view::WordPair]) -> String {
    if words.is_empty() {
        return esc(text);
    }

    let mut by_phrase: HashMap<String, &view::WordPair> = HashMap::new();
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

        let mut matched: Option<(usize, &view::WordPair)> = None;
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
    use super::mark_terms;
    use crate::view::WordPair;

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
