//! The `/` route: the topic list, terms summary card, and check report.
use super::decisions::chosen_sentence;
use super::html::{banner_html, esc, neighbours_html, page, plural};
use super::percent_encode;
use super::terms::unpaired_count;
use crate::view::{self, QState, QuestionView, Snapshot};

pub(super) fn render_index(snap: &Snapshot, m: Option<&String>, token: &str) -> String {
    let mut body = String::new();
    body.push_str(&banner_html(m));

    let open_count = view::open_questions(snap).len();
    let topic_count = snap.files.len();
    body.push_str("<header class=\"lead\">\n<h1>Overview</h1>\n");
    body.push_str(&format!(
        "<p class=\"summary\">{} across {}.</p>\n",
        plural(open_count, "open question", "open questions"),
        plural(topic_count, "topic", "topics")
    ));
    if open_count > 0 {
        body.push_str("<p><a class=\"primary\" href=\"/next\">Answer the next question</a></p>\n");
    }
    body.push_str("</header>\n");

    for f in &snap.files {
        body.push_str("<article class=\"topic\">\n");
        body.push_str(&format!("<h2>{}</h2>\n", esc(&f.topic)));
        body.push_str(&format!("<p class=\"path\">{}</p>\n", esc(&f.path)));

        let mut awaiting: Vec<&str> = Vec::new();
        if !f.plan_approved {
            awaiting.push("plan");
        }
        if !f.context_approved {
            awaiting.push("context");
        }
        if !awaiting.is_empty() {
            body.push_str(&format!(
                "<p class=\"state waiting\">The {} of this file is not approved yet.</p>\n",
                awaiting.join(" and ")
            ));
            // Per "Does show print the neighbours of a question with the
            // reader's word?" (97c679f6-e8e6-4970-b547-940477753d7d A): the
            // owner sees each heading with its neighbours, the reader's word
            // and the agent's line where the Approve button is.
            let unasked: Vec<&QuestionView> = f
                .questions
                .iter()
                .filter(|q| matches!(q.state, QState::Skeleton | QState::Unasked))
                .collect();
            if !unasked.is_empty() {
                body.push_str("<ul class=\"unasked-list\">\n");
                for q in &unasked {
                    body.push_str(&format!("<li>{}", esc(&q.heading)));
                    body.push_str(&neighbours_html(&q.neighbours));
                    body.push_str("</li>\n");
                }
                body.push_str("</ul>\n");
            }
            body.push_str(&format!(
                "<form method=\"post\" action=\"/approve\" class=\"inline\"><input type=\"hidden\" name=\"file\" value=\"{}\"><input type=\"hidden\" name=\"t\" value=\"{}\"><button type=\"submit\">Approve {}</button></form>\n",
                esc(&f.path),
                esc(token),
                esc(&f.topic)
            ));
        }
        if let Some(t) = f.open_tier {
            let mut s = format!("Tier {} is open", t);
            if let Some(w) = &f.waits_on {
                s.push_str(&format!(", waiting on <q>{}</q>", esc(w)));
            }
            s.push('.');
            body.push_str(&format!("<p class=\"state\">{}</p>\n", s));
        }
        if f.done {
            body.push_str("<p class=\"state done\">Every approved heading is answered.</p>\n");
        }

        let open: Vec<&QuestionView> = f
            .questions
            .iter()
            .filter(|q| q.state == QState::Open)
            .collect();
        if !open.is_empty() {
            body.push_str("<ol class=\"open-list\">\n");
            for q in &open {
                let id = q.id.clone().unwrap_or_default();
                body.push_str(&format!(
                    "<li><a href=\"/next?show={}\">{}</a></li>\n",
                    percent_encode(&id),
                    esc(&q.heading)
                ));
            }
            body.push_str("</ol>\n");
        }

        let refused: Vec<&QuestionView> = f
            .questions
            .iter()
            .filter(|q| q.state == QState::Refused)
            .collect();
        if !refused.is_empty() {
            body.push_str("<ul class=\"refused-list\">\n");
            for q in &refused {
                body.push_str(&format!(
                    "<li><span class=\"label refused\">refused</span> {}</li>\n",
                    esc(&q.heading)
                ));
            }
            body.push_str("</ul>\n");
        }

        let answered: Vec<&QuestionView> = f
            .questions
            .iter()
            .filter(|q| q.state == QState::Answered)
            .collect();
        if !answered.is_empty() {
            body.push_str(&format!(
                "<details class=\"answered\"><summary>{}</summary>\n<dl>\n",
                plural(answered.len(), "answered question", "answered questions")
            ));
            for q in &answered {
                let sentence = chosen_sentence(q);
                body.push_str(&format!(
                    "<dt>{}</dt><dd>{}</dd>\n",
                    esc(&q.heading),
                    esc(sentence)
                ));
            }
            body.push_str("</dl></details>\n");
        }

        body.push_str("</article>\n");
    }

    if let Some(t) = &snap.terms {
        body.push_str("<article class=\"topic\">\n<h2>Terms</h2>\n");
        body.push_str(&format!("<p class=\"path\">{}</p>\n", esc(&t.path)));
        body.push_str("<div class=\"summary-card\">");
        body.push_str(&format!(
            "<span class=\"count\">{}</span>",
            plural(t.count, "term", "terms")
        ));
        if t.approved {
            body.push_str("<span class=\"state done\" style=\"margin:0\">approved</span>");
        } else {
            body.push_str(
                "<span class=\"state waiting\" style=\"margin:0\">not approved yet</span>",
            );
        }
        let unpaired = unpaired_count(t);
        if unpaired > 0 {
            body.push_str(&format!(
                "<span class=\"muted\">{} paired by nothing</span>",
                unpaired
            ));
        }
        body.push_str("<a class=\"more\" href=\"/terms\">Open the terms &rarr;</a>");
        body.push_str("</div>\n");
        if !t.approved {
            body.push_str(&format!(
                "<form method=\"post\" action=\"/approve\" class=\"inline\"><input type=\"hidden\" name=\"file\" value=\"{}\"><input type=\"hidden\" name=\"t\" value=\"{}\"><button type=\"submit\">Approve terms</button></form>\n",
                esc(&t.path),
                esc(token)
            ));
        }
        if !t.errors.is_empty() {
            body.push_str("<ul class=\"failures\">\n");
            for e in &t.errors {
                body.push_str(&format!("<li>{}</li>\n", esc(e)));
            }
            body.push_str("</ul>\n");
        }
        body.push_str("</article>\n");
    }

    body.push_str("<article class=\"topic check\">\n<h2>Check</h2>\n");
    if snap.check.failures.is_empty() {
        body.push_str("<p class=\"state done\">Nothing failing.</p>\n");
    } else {
        body.push_str(&format!(
            "<p class=\"state failing\">{}.</p>\n",
            plural(snap.check.failures.len(), "failure", "failures")
        ));
        body.push_str("<ul class=\"failures\">\n");
        for f in &snap.check.failures {
            body.push_str(&format!("<li>{}</li>\n", esc(f)));
        }
        body.push_str("</ul>\n");
    }
    if !snap.check.notes.is_empty() {
        body.push_str("<ul class=\"notes\">\n");
        for n in &snap.check.notes {
            body.push_str(&format!("<li>{}</li>\n", esc(n)));
        }
        body.push_str("</ul>\n");
    }
    body.push_str("</article>\n");

    body.push_str("<details class=\"unban\"><summary>Unban a phrase</summary>\n");
    body.push_str(&format!(
        "<form method=\"post\" action=\"/unban\"><textarea name=\"lines\" rows=\"4\" placeholder=\"One phrase per line\"></textarea><input type=\"hidden\" name=\"t\" value=\"{}\"><button type=\"submit\">Unban</button></form>\n",
        esc(token)
    ));
    body.push_str("</details>\n");

    page("Overview", "/", &body, false)
}
