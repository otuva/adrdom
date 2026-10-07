//! The `/decisions` route: the current, answered decision list.
use super::html::{esc, page, plural};
use crate::view::{QuestionStatus, QuestionView, Snapshot};

pub(super) fn render_decisions(snap: &Snapshot) -> String {
    let mut body = String::new();
    let total: usize = snap
        .files
        .iter()
        .map(|f| {
            f.questions
                .iter()
                .filter(|q| q.state == QuestionStatus::Answered)
                .count()
        })
        .sum();
    body.push_str("<header class=\"lead\">\n<h1>Current decisions</h1>\n");
    body.push_str(&format!(
        "<p class=\"summary\">{}.</p>\n</header>\n",
        plural(total, "decision", "decisions")
    ));
    for f in &snap.files {
        let answered: Vec<&QuestionView> = f
            .questions
            .iter()
            .filter(|q| q.state == QuestionStatus::Answered)
            .collect();
        if answered.is_empty() {
            continue;
        }
        body.push_str("<article class=\"topic\">\n");
        body.push_str(&format!("<h2>{}</h2>\n", esc(&f.topic)));
        body.push_str("<dl class=\"decisions\">\n");
        for q in &answered {
            let sentence = chosen_sentence(q);
            let tests = match q.tests {
                0 => "untested".to_string(),
                n => plural(n, "test", "tests"),
            };
            body.push_str(&format!(
                "<dt>{}</dt><dd><span class=\"sentence\">{}</span><span class=\"meta\">{}",
                esc(&q.heading),
                esc(sentence),
                tests
            ));
            if !q.supersedes.is_empty() {
                body.push_str(&format!(", supersedes {}", esc(&q.supersedes.join(", "))));
            }
            body.push_str("</span></dd>\n");
        }
        body.push_str("</dl>\n</article>\n");
    }
    page("Decisions", "/decisions", &body, false)
}

pub(super) fn chosen_sentence(q: &QuestionView) -> &str {
    q.options
        .iter()
        .find(|o| Some(o.letter) == q.answer)
        .map(|o| o.sentence.as_str())
        .unwrap_or("")
}
