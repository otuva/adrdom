//! The `/next` route: one asked question at a time with its options, the
//! terms its words pair, what it stands on, the moves that name it, and the
//! other open questions. The first question `answer` takes comes first;
//! `after` moves on from one the owner skipped or answered, `show` names one.
use super::data::{self, Q};
use super::html::{banner, esc, neighbours, page, plural};
use super::overview::{notes, open_item};
use super::percent_encode;
use super::words::{self, mark_terms, WordPair};
use crate::commands::show::{self, Records};
use crate::out::Line;
use crate::record::{self, Dep, Section};
use std::collections::HashMap;

pub(super) fn render_next(r: &Records, last: &[Line], after: Option<&String>, show: Option<&String>, token: &str) -> String {
    let mut body = banner(last);
    let files = data::files(r, false);
    let all: Vec<&Q> = files.iter().flat_map(|f| f.qs.iter()).filter(|q| q.listed()).collect();

    let Some(q) = pick_next(&all, after.map(String::as_str), show.map(String::as_str)) else {
        // An asked question `answer` would refuse waits on another: nothing
        // is wrong, but nothing can be answered yet either.
        let waiting = !all.is_empty();
        body.push_str(&format!(
            "<header class=\"lead\">\n<h1>{}</h1>\n",
            if waiting { "No question can be answered yet" } else { "Nothing to answer" }
        ));
        body.push_str("</header>\n");
        if waiting {
            body.push_str(&queue(&all, ""));
        }
        body.push_str("<p><a href=\"/\">Back to the overview</a></p>\n");
        return page("Answer", "/next", &body, true);
    };

    let id = q.id().to_string();
    let rf = q.rf;
    let sec = q.sec;
    let (words, no_entry) = words::section_words(rf, sec, &r.terms_list);
    let position = all.iter().position(|x| x.id() == id).map(|p| p + 1);

    body.push_str("<article class=\"focus\">\n<p class=\"where\">");
    body.push_str(&esc(rf.topic.as_deref().unwrap_or(&rf.rel)));
    if let Some(t) = rf.tier_of(q.idx) {
        body.push_str(&format!(", tier {}", t));
    }
    body.push_str(&format!(" @{}", esc(&record::version_of(&rf.lines, sec))));
    if let Some(p) = position {
        body.push_str(&format!(". Question {} of {} open", p, all.len()));
    }
    body.push_str(".</p>\n");
    body.push_str(&format!("<h1>{}</h1>\n", mark_terms(q.heading(), &words)));

    // What the question waits on, when it waits: the owner's answer is the
    // form below.
    let waits = notes(q);
    if !waits.is_empty() {
        body.push_str(&format!("<div class=\"lines\">\n{}</div>\n", waits));
    }

    let depends = depends_text(r, sec);
    if !depends.is_empty() {
        body.push_str(&format!("<p class=\"stands-on\">Stands on: {}</p>\n", esc(&depends.join(", "))));
    }
    if !sec.supersedes.is_empty() {
        let names: Vec<String> =
            sec.supersedes.iter().map(|id| r.names.get(id).cloned().unwrap_or_else(|| id.clone())).collect();
        body.push_str(&format!("<p class=\"stands-on\">Supersedes: {}</p>\n", esc(&names.join(", "))));
    }
    body.push_str(&neighbours(&show::neighbours_of(r, rf, q.heading())));

    let problem = body_field(rf, sec, "Problem: ");
    let standard = body_field(rf, sec, "Standard: ");
    if problem.is_some() || standard.is_some() {
        body.push_str("<dl class=\"framing\">\n");
        if let Some(p) = &problem {
            body.push_str(&format!("<dt>Problem</dt><dd>{}</dd>\n", mark_terms(p, &words)));
        }
        if let Some(s) = &standard {
            body.push_str(&format!("<dt>Standard</dt><dd>{}</dd>\n", mark_terms(s, &words)));
        }
        body.push_str("</dl>\n");
    }
    body.push_str(&term_aside(&words, &no_entry));

    // An asked question `answer` would refuse (reached by a link or a
    // `show` address): the moves above name what it waits on, and no form.
    let Some((_, version)) = q.answer() else {
        body.push_str(&format!(
            "<p><a class=\"skip\" href=\"/next?after={}\">Skip this one</a></p>\n",
            percent_encode(&id)
        ));
        body.push_str(&queue(&all, &id));
        body.push_str("</article>\n");
        return page("Answer", "/next", &body, true);
    };

    body.push_str(&format!(
        "<form method=\"post\" action=\"/answer\" id=\"answer-form\">\n<input type=\"hidden\" name=\"id\" value=\"{}\">\n<input type=\"hidden\" name=\"v\" value=\"{}\">\n<input type=\"hidden\" name=\"t\" value=\"{}\">\n",
        esc(&id),
        esc(version),
        esc(token)
    ));
    body.push_str("<div class=\"options\" role=\"radiogroup\">\n");
    for (i, (opt, (_, why, costs))) in sec.options.iter().zip(option_details(rf, sec)).enumerate() {
        body.push_str(&format!(
            "<label class=\"option\" for=\"opt-{l}\"><input type=\"radio\" name=\"answer\" id=\"opt-{l}\" value=\"{l}\" required{c}><span class=\"key\">{l}</span><span class=\"text\"><span class=\"sentence\">{s}</span>",
            l = opt.letter,
            c = if i == 0 { " checked" } else { "" },
            s = mark_terms(opt.sentence.as_deref().unwrap_or(""), &words),
        ));
        if why.is_some() || costs.is_some() {
            body.push_str("<dl class=\"rationale\">");
            if let Some(w) = &why {
                body.push_str(&format!("<dt>Why</dt><dd>{}</dd>", mark_terms(w, &words)));
            }
            if let Some(c) = &costs {
                body.push_str(&format!("<dt>Costs</dt><dd>{}</dd>", mark_terms(c, &words)));
            }
            body.push_str("</dl>");
        }
        body.push_str("</span></label>\n");
    }
    body.push_str("</div>\n");
    // The Refuse button shares the `answer` field with the letters: it posts
    // `answer=refuse` when it, not a letter, submits the form (per "Does the owner answer a
    // question with a refusal?"). As on the old page, the first option is
    // picked and the button names the letter picked.
    body.push_str(&format!(
        "<p class=\"act\"><button type=\"submit\" class=\"answer\" id=\"answer-button\">Answer <span id=\"answer-letter\">{first}</span></button> <button type=\"submit\" name=\"answer\" value=\"refuse\" class=\"refuse\" id=\"refuse-button\" formnovalidate>Refuse</button> <a class=\"skip\" href=\"/next?after={}\">Skip this one</a></p>\n",
        percent_encode(&id),
        first = sec.options.first().map(|o| o.letter).unwrap_or('A')
    ));
    body.push_str("</form>\n");
    body.push_str("<p class=\"key-hint\">Press a letter to choose, <kbd>Enter</kbd> to answer, <kbd>S</kbd> to skip.</p>\n");
    body.push_str(&queue(&all, &id));
    body.push_str("</article>\n");

    body.push_str(&format!(
        r#"<script>
(function() {{
  var form = document.getElementById('answer-form');
  var letter = document.getElementById('answer-letter');
  function sync() {{
    var chosen = form.querySelector('input[name=answer]:checked');
    if (letter) {{ letter.textContent = chosen ? chosen.value : ''; }}
  }}
  form.addEventListener('change', sync);
  sync();
  document.addEventListener('keydown', function(e) {{
    if (e.altKey || e.ctrlKey || e.metaKey) return;
    var tag = (document.activeElement && document.activeElement.tagName) || '';
    if (tag === 'INPUT' || tag === 'TEXTAREA') {{
      if (e.key === 'Enter' && form.querySelector('input[name=answer]:checked')) {{ form.requestSubmit(); }}
      return;
    }}
    var key = e.key.toUpperCase();
    if (key.length === 1 && key >= 'A' && key <= 'E') {{
      var radio = document.getElementById('opt-' + key);
      if (radio) {{ radio.checked = true; sync(); }}
    }} else if (key === 'ENTER') {{
      if (form.querySelector('input[name=answer]:checked')) {{ form.requestSubmit(); }}
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

/// The open questions of every file: how many, and a link to each one
/// `answer` takes, the others with what they wait on.
fn queue(all: &[&Q], current: &str) -> String {
    let mut s = format!("<section class=\"queue\">\n<h2>{}</h2>\n<ol class=\"open-list\">\n", plural(all.len(), "open question", "open questions"));
    for q in all {
        s.push_str(&open_item(q, q.id() == current));
    }
    s.push_str("</ol>\n</section>\n");
    s
}

/// The terms the question's words pair, each once with the forms the
/// question uses, and the words that pair none.
fn term_aside(words: &[WordPair], no_entry: &[String]) -> String {
    let mut s = String::from("<aside class=\"tq\">\n<h2>Terms in this question</h2>\n");
    if words.is_empty() {
        s.push_str("<p class=\"meta\">No words pair a term.</p>\n");
    } else {
        // Group by term name, first-seen order.
        let mut order: Vec<&str> = Vec::new();
        let mut groups: HashMap<&str, (&str, Vec<&str>)> = HashMap::new();
        for w in words {
            let entry = groups.entry(w.term_name.as_str()).or_insert_with(|| {
                order.push(w.term_name.as_str());
                (w.definition.as_str(), Vec::new())
            });
            if w.word != w.term_name {
                entry.1.push(w.word.as_str());
            }
        }
        s.push_str("<dl>\n");
        for name in &order {
            let (def, forms) = &groups[name];
            s.push_str(&format!("<dt>{}", esc(name)));
            if !forms.is_empty() {
                s.push_str(&format!("<span class=\"forms\">{}</span>", esc(&forms.join(", "))));
            }
            s.push_str(&format!("</dt><dd>{}</dd>\n", esc(def)));
        }
        s.push_str("</dl>\n");
    }
    if !no_entry.is_empty() {
        s.push_str(&format!("<p class=\"none\"><b>No term:</b> {}</p>\n", esc(&no_entry.join(", "))));
    }
    s.push_str("</aside>\n");
    s
}

/// The `depends:` entries of `sec` as words: an identifier is turned back
/// into the heading or the term it names, the answer set kept as written.
fn depends_text(r: &Records, sec: &Section) -> Vec<String> {
    sec.depends
        .iter()
        .map(|d| match d {
            Dep::Text { text, set } => format!("{}{}", text, record::set_suffix(set)),
            Dep::Id { id, set } => format!("{}{}", r.names.get(id).unwrap_or(id), record::set_suffix(set)),
        })
        .collect()
}

/// The text after `prefix` (`Problem: `) on the first matching line of the
/// body of `sec`.
fn body_field(rf: &record::RecordFile, sec: &Section, prefix: &str) -> Option<String> {
    rf.lines[sec.start..sec.end.min(rf.lines.len())].iter().find_map(|l| l.strip_prefix(prefix).map(|v| v.trim().to_string()))
}

/// For each option of `sec`, its letter and the `  Why: ` and `  Costs: `
/// lines that follow it.
fn option_details(rf: &record::RecordFile, sec: &Section) -> Vec<(char, Option<String>, Option<String>)> {
    let lines = &rf.lines;
    let mut out = Vec::new();
    for opt in &sec.options {
        let (mut why, mut costs) = (None, None);
        for line in &lines[(opt.line + 1).min(lines.len())..sec.end.min(lines.len())] {
            if let Some((kind, v)) = record::detail_line(line) {
                match kind {
                    record::Detail::Why if why.is_none() => why = Some(v.trim().to_string()),
                    record::Detail::Costs if costs.is_none() => costs = Some(v.trim().to_string()),
                    _ => {}
                }
                continue;
            }
            if line.trim().is_empty() {
                continue;
            }
            break;
        }
        out.push((opt.letter, why, costs));
    }
    out
}

/// The question the page opens on: the one `show` names, else the first
/// `answer` takes after `after` (forward in file order, then from the top;
/// the skipped one comes last), else the first it takes.
fn pick_next<'a>(all: &[&'a Q<'a>], after: Option<&str>, show: Option<&str>) -> Option<&'a Q<'a>> {
    if let Some(id) = show {
        if let Some(q) = all.iter().find(|q| q.id() == id) {
            return Some(q);
        }
    }
    if let Some(id) = after {
        if let Some(pos) = all.iter().position(|q| q.id() == id) {
            return all[pos + 1..].iter().chain(all[..=pos].iter()).find(|q| q.answerable()).copied();
        }
    }
    all.iter().find(|q| q.answerable()).copied()
}

