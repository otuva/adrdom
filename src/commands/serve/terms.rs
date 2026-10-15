//! The `/terms` route: every entry of the terms file with its status line
//! and the decisions that stand on it, a filter box, and the approval of
//! the file with the change it signs above its button.
use super::html::{banner, esc, page, plural, st};
use crate::commands::show::{self, Records};
use crate::out::{self, Line, Status};
use crate::repo;
use crate::status;

pub(super) fn render_terms(r: &Records, last: &[Line], token: &str) -> String {
    let mut body = banner(last);
    body.push_str("<header class=\"lead\">\n<h1>Terms</h1>\n");

    let Some(tf) = &r.terms_file else {
        // The finding `show` prints for a terms file that cannot be read.
        match r.terms_unreadable_finding() {
            Some(l) => body.push_str(&format!("{}\n", out::html(&l))),
            None => body.push_str(&format!("{}\n", out::html(&out::refusal(
                crate::rules::Command::Show,
                &crate::msg::read_failed(repo::TERMS_PATH, "no such file", crate::msg::Next::agent("names a file that exists")),
            )))),
        }
        body.push_str("</header>\n");
        return page("Terms", "/terms", &body, false);
    };

    let (file_status, report) = show::terms_report(r, tf);
    let dependents: Vec<usize> = tf.terms.iter().map(|t| show::term_dependents(r, t).len()).collect();
    let unpaired = tf.terms.iter().zip(&dependents).filter(|(t, n)| !t.retired && **n == 0).count();
    let mut summary = format!("{} in {}.", plural(tf.terms.len(), "term", "terms"), esc(repo::TERMS_PATH));
    if unpaired > 0 {
        summary.push_str(&format!(" {} paired by nothing.", unpaired));
    }
    body.push_str(&format!("<p class=\"summary\">{}</p>\n", summary));
    let st_line = Line::Status { subject: repo::TERMS_PATH.to_string(), status: Status::File(file_status), mark: None };
    body.push_str(&format!(
        "<div class=\"state {}\">{}</div>\n",
        if file_status.awaits_approval() { "waiting" } else { "done" },
        st(&st_line, false, None)
    ));

    body.push_str("<div class=\"tools\">");
    body.push_str(
        "<input class=\"filter\" type=\"search\" placeholder=\"Filter by word, alias, or definition\" aria-label=\"Filter terms\">",
    );
    body.push_str("</div>\n");
    body.push_str(&super::overview::terms_moves(r, tf, &report.moves, "Approve the terms", token));
    body.push_str("</header>\n");

    if !report.findings.is_empty() {
        let fl: Vec<Line> = report.findings.iter().cloned().map(Line::Finding).collect();
        body.push_str(&format!("<div class=\"failures\">\n{}</div>\n", super::html::lines(&fl)));
    }

    body.push_str("<ul class=\"terms-list\" id=\"terms\">\n");
    for (t, n) in tf.terms.iter().zip(&dependents) {
        let aliases = t.aliases.join(", ");
        let data_k = format!("{} {} {}", t.name, aliases, t.definition).to_lowercase();
        body.push_str(&format!(
            "<li{} data-k=\"{}\"><div class=\"name\">{}",
            if t.retired { " class=\"retired\"" } else { "" },
            esc(&data_k),
            esc(&t.name)
        ));
        if !t.aliases.is_empty() {
            body.push_str(&format!("<span class=\"forms\">{}</span>", esc(&aliases)));
        }
        body.push_str("</div><div>");
        body.push_str(&format!("<p class=\"def\">{}</p>", esc(&t.definition)));
        if *n > 0 {
            body.push_str(&format!("<p class=\"use\">Paired by <a href=\"/decisions\">{}</a></p>", plural(*n, "decision", "decisions")));
        } else {
            body.push_str("<p class=\"use\">Paired by nothing yet</p>");
        }
        body.push_str("</div></li>\n");
    }
    body.push_str("</ul>\n");
    // The status of each entry, folded: the lines `adr show` prints for them.
    body.push_str(&format!(
        "<details class=\"answered\"><summary>{}</summary>\n",
        plural(tf.terms.len(), "entry status", "entry statuses")
    ));
    for t in &tf.terms {
        let line = Line::Status {
            subject: t.name.clone(),
            status: Status::Term(status::term_status(&r.led, tf, t)),
            mark: None,
        };
        body.push_str(&format!("<div>{}</div>\n", st(&line, false, None)));
    }
    body.push_str("</details>\n");
    body.push_str("<p class=\"no-match\" id=\"no-match\" hidden>No term matches.</p>\n");
    body.push_str(
        r#"<script>
(function(){var f=document.querySelector('input.filter'),items=[].slice.call(document.querySelectorAll('#terms li')),none=document.getElementById('no-match');
f.addEventListener('input',function(){var q=f.value.trim().toLowerCase(),n=0;items.forEach(function(li){var hit=!q||li.dataset.k.indexOf(q)>=0;li.hidden=!hit;if(hit)n++;});none.hidden=n>0;});})();
</script>
"#,
    );

    page("Terms", "/terms", &body, false)
}
