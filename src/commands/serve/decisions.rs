//! The `/decisions` route: every current decision, its status line, the
//! option picked and how many tests tie to it.
use super::html::{banner, esc, page, plural, st};
use crate::commands::show::Records;
use crate::out::Line;
use crate::status::{self, QuestionStatus};

pub(super) fn render_decisions(r: &Records, last: &[Line]) -> String {
    let mut body = banner(last);
    let answered = |rf: &crate::record::RecordFile| -> Vec<usize> {
        (0..rf.sections.len())
            .filter(|&i| status::question_status(&r.led, &rf.sections[i]) == QuestionStatus::Answered)
            .collect()
    };
    // The current decisions of a topic file removed from disk stand until a
    // plan change supersedes them: read from its approved version, as `show`
    // lists them after the topic files.
    let removed: Vec<std::sync::Arc<crate::record::RecordFile>> = r
        .removed_topic_files()
        .iter()
        .filter_map(|rel| crate::rules::latest_approved_version(&r.root, rel, &r.led))
        .collect();
    let files: Vec<&crate::record::RecordFile> = r.topic_files.iter().chain(removed.iter().map(|a| &**a)).collect();
    let total: usize = files.iter().map(|rf| answered(rf).len()).sum();
    body.push_str("<header class=\"lead\">\n<h1>Current decisions</h1>\n");
    body.push_str(&format!("<p class=\"summary\">{}.</p>\n</header>\n", plural(total, "decision", "decisions")));
    for rf in files {
        let idx = answered(rf);
        if idx.is_empty() {
            continue;
        }
        body.push_str("<article class=\"topic\">\n");
        body.push_str(&format!("<h2>{}</h2>\n<dl class=\"decisions\">\n", esc(rf.topic.as_deref().unwrap_or(&rf.rel))));
        for i in idx {
            let sec = &rf.sections[i];
            let line = Line::Status {
                subject: sec.heading_text.clone(),
                status: crate::commands::show::question_status_of(&r.led, rf, sec),
                mark: None,
            };
            let tests = sec.id.as_ref().and_then(|id| r.tests.get(id)).map(|v| v.len()).unwrap_or(0);
            let tests = match tests {
                0 => "untested".to_string(),
                n => plural(n, "test", "tests"),
            };
            body.push_str(&format!(
                "<dt>{}</dt><dd><span class=\"sentence\">{}</span><span class=\"meta\">{}",
                st(&line, false, None),
                esc(sec.chosen_sentence().unwrap_or("")),
                tests
            ));
            if !sec.supersedes.is_empty() {
                let names: Vec<String> =
                    sec.supersedes.iter().map(|id| r.names.get(id).cloned().unwrap_or_else(|| id.clone())).collect();
                body.push_str(&format!(", supersedes {}", esc(&names.join(", "))));
            }
            body.push_str("</span></dd>\n");
        }
        body.push_str("</dl>\n</article>\n");
    }
    page("Decisions", "/decisions", &body, false)
}
