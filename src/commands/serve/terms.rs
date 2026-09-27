//! The `/terms` route: the terms list, filter box, and approval state.
use super::html::{banner_html, esc, page, plural};
use crate::view::{self, Snapshot};

/// The number of current entries paired by no current decision: not
/// retired, and with a dependents count of zero.
pub(super) fn unpaired_count(t: &view::TermsView) -> usize {
    t.entries
        .iter()
        .filter(|e| !e.retired && e.dependents == 0)
        .count()
}

pub(super) fn render_terms(snap: &Snapshot, m: Option<&String>, token: &str) -> String {
    let mut body = String::new();
    body.push_str(&banner_html(m));
    body.push_str("<header class=\"lead\">\n<h1>Terms</h1>\n");

    let Some(t) = &snap.terms else {
        body.push_str("<p class=\"state waiting\">No terms file yet.</p>\n</header>\n");
        return page("Terms", "/terms", &body, false);
    };

    let unpaired = unpaired_count(t);
    let mut extra: Vec<String> = Vec::new();
    if !t.approved {
        extra.push("waiting for approval".to_string());
    }
    if unpaired > 0 {
        extra.push(format!("{} paired by nothing", unpaired));
    }
    let mut summary = format!("{} in {}.", plural(t.count, "term", "terms"), esc(&t.path));
    if !extra.is_empty() {
        summary.push(' ');
        summary.push_str(&extra.join(", "));
        summary.push('.');
    }
    body.push_str(&format!("<p class=\"summary\">{}</p>\n", summary));

    body.push_str("<div class=\"tools\">");
    body.push_str(
        "<input class=\"filter\" type=\"search\" placeholder=\"Filter by word, alias, or definition\" aria-label=\"Filter terms\">",
    );
    if !t.approved {
        body.push_str(&format!(
            "<form method=\"post\" action=\"/approve\" class=\"inline\" style=\"margin:0\"><input type=\"hidden\" name=\"file\" value=\"{}\"><input type=\"hidden\" name=\"t\" value=\"{}\"><button type=\"submit\" class=\"answer\">Approve the terms</button></form>",
            esc(&t.path),
            esc(token)
        ));
    }
    body.push_str("</div>\n");
    body.push_str("</header>\n");

    if !t.errors.is_empty() {
        body.push_str("<ul class=\"failures\">\n");
        for e in &t.errors {
            body.push_str(&format!("<li>{}</li>\n", esc(e)));
        }
        body.push_str("</ul>\n");
    }

    body.push_str("<ul class=\"terms-list\" id=\"terms\">\n");
    for e in &t.entries {
        let aliases = e.aliases.join(", ");
        let data_k = format!("{} {} {}", e.name, aliases, e.definition).to_lowercase();
        body.push_str(&format!(
            "<li{} data-k=\"{}\"><div class=\"name\">{}",
            if e.retired { " class=\"retired\"" } else { "" },
            esc(&data_k),
            esc(&e.name)
        ));
        if e.retired {
            body.push_str("<span class=\"badge retired\">retired</span>");
        }
        if !e.aliases.is_empty() {
            body.push_str(&format!("<span class=\"forms\">{}</span>", esc(&aliases)));
        }
        body.push_str("</div><div>");
        body.push_str(&format!("<p class=\"def\">{}</p>", esc(&e.definition)));
        if e.dependents > 0 {
            body.push_str(&format!(
                "<p class=\"use\">Paired by <a href=\"/decisions\">{}</a></p>",
                plural(e.dependents, "decision", "decisions")
            ));
        } else {
            body.push_str("<p class=\"use\">Paired by nothing yet</p>");
        }
        body.push_str("</div></li>\n");
    }
    body.push_str("</ul>\n");
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
