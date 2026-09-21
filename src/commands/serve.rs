//! The owner's page (7.x): served by the binary on loopback, token-gated.
use crate::actions;
use crate::repo;
use crate::view::{self, FileView, QState, QuestionView, Snapshot};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use tiny_http::{Header, Method, Request, Response, Server};

pub fn run(port: u16) -> i32 {
    if !repo::owner_authorized() {
        eprintln!(
            "serve: refusing: owner authorization required (interactive TTY, no CLAUDECODE)"
        );
        return 2;
    }

    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("serve: not inside a git repository");
            return 1;
        }
    };

    let server = match Server::http(("127.0.0.1", port)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("serve: failed to bind 127.0.0.1:{}: {}", port, e);
            return 1;
        }
    };

    let bound_port = server
        .server_addr()
        .to_ip()
        .map(|a| a.port())
        .unwrap_or(port);

    let token = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );

    println!("open: http://127.0.0.1:{}/?t={}", bound_port, token);
    let _ = std::io::stdout().flush();

    for request in server.incoming_requests() {
        handle(request, &root, &token);
    }

    0
}

// --- request handling -----------------------------------------------------

enum Reply {
    Html(u16, String),
    Redirect(String, Option<String>),
    Status(u16),
}

fn handle(mut request: Request, root: &Path, token: &str) {
    let method = request.method().clone();
    let raw_url = request.url().to_string();
    let (path, query_string) = split_path_query(&raw_url);
    let query = parse_form(&query_string);

    if path == "/favicon.ico" {
        send(request, Reply::Status(204));
        return;
    }

    let cookie_token = cookie_value(request.headers(), "adr_t");

    let body = if method == Method::Post {
        let mut s = String::new();
        let _ = request.as_reader().read_to_string(&mut s);
        s
    } else {
        String::new()
    };
    let form = parse_form(&body);

    // GET with a matching `?t=` token: set the cookie and redirect to the
    // same path with `t` stripped out of the query string.
    if method == Method::Get {
        if let Some(t) = query.get("t") {
            if token_eq(t, token) {
                let dest = strip_query_key(&path, &query_string, "t");
                send(
                    request,
                    Reply::Redirect(dest, Some(cookie_header_value(token))),
                );
                return;
            }
        }
    }

    let cookie_ok = cookie_token.as_deref().is_some_and(|c| token_eq(c, token));
    if !cookie_ok {
        send(request, Reply::Html(403, forbidden_page()));
        return;
    }
    if method == Method::Post && !form.get("t").is_some_and(|t| token_eq(t, token)) {
        send(request, Reply::Html(403, forbidden_page()));
        return;
    }

    let reply = match (method, path.as_str()) {
        (Method::Get, "/") => {
            let snap = view::snapshot(root);
            Reply::Html(200, render_index(&snap, query.get("m"), token))
        }
        (Method::Get, "/next") => {
            let snap = view::snapshot(root);
            Reply::Html(
                200,
                render_next(&snap, query.get("after"), query.get("show"), query.get("m"), token),
            )
        }
        (Method::Get, "/decisions") => {
            let snap = view::snapshot(root);
            Reply::Html(200, render_decisions(&snap))
        }
        (Method::Post, "/approve") => handle_approve(root, &form),
        (Method::Post, "/lock") => handle_lock(root, &form),
        (Method::Post, "/unban") => handle_unban(root, &form),
        _ => Reply::Html(404, simple_page("Not found", "<p>Not found.</p>")),
    };

    send(request, reply);
}

fn send(request: Request, reply: Reply) {
    match reply {
        Reply::Html(code, body) => {
            let resp = Response::from_string(body)
                .with_status_code(code)
                .with_header(header("Content-Type", "text/html; charset=utf-8"));
            let _ = request.respond(resp);
        }
        Reply::Redirect(location, cookie) => {
            let mut resp = Response::empty(302).with_header(header("Location", &location));
            if let Some(c) = cookie {
                resp.add_header(header("Set-Cookie", &c));
            }
            let _ = request.respond(resp);
        }
        Reply::Status(code) => {
            let _ = request.respond(Response::empty(code));
        }
    }
}

fn cookie_header_value(token: &str) -> String {
    format!("adr_t={}; HttpOnly; SameSite=Strict; Path=/", token)
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("valid header")
}

// --- actions ---------------------------------------------------------------

fn handle_approve(root: &Path, form: &HashMap<String, String>) -> Reply {
    let file = form.get("file").cloned().unwrap_or_default();
    let banner = match actions::do_approve(root, &root.join(&file)) {
        Ok(o) => o.summary,
        Err(e) => e,
    };
    Reply::Redirect(format!("/?m={}", percent_encode(&banner)), None)
}

fn handle_lock(root: &Path, form: &HashMap<String, String>) -> Reply {
    let id = form.get("id").cloned().unwrap_or_default();
    let letter = form
        .get("answer")
        .and_then(|s| s.trim().chars().next())
        .map(|c| c.to_ascii_uppercase());

    let Some(letter) = letter else {
        let banner = "serve: missing answer selection";
        return Reply::Redirect(
            format!("/next?show={}&m={}", percent_encode(&id), percent_encode(banner)),
            None,
        );
    };

    match actions::do_lock(root, &id, letter) {
        Ok(o) => Reply::Redirect(
            format!("/next?after={}&m={}", percent_encode(&id), percent_encode(&o.summary)),
            None,
        ),
        Err(e) => Reply::Redirect(
            format!("/next?show={}&m={}", percent_encode(&id), percent_encode(&e)),
            None,
        ),
    }
}

fn handle_unban(root: &Path, form: &HashMap<String, String>) -> Reply {
    let lines = form.get("lines").cloned().unwrap_or_default();
    let banner = match actions::do_unban(root, &lines) {
        Ok(o) => o.summary,
        Err(e) => e,
    };
    Reply::Redirect(format!("/?m={}", percent_encode(&banner)), None)
}

// --- pages -------------------------------------------------------------

fn render_index(snap: &Snapshot, m: Option<&String>, token: &str) -> String {
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
            body.push_str(&format!(
                "<form method=\"post\" action=\"/approve\" class=\"inline\"><input type=\"hidden\" name=\"file\" value=\"{}\"><input type=\"hidden\" name=\"t\" value=\"{}\"><button type=\"submit\">Approve {}</button></form>\n",
                esc(&f.path),
                esc(token),
                esc(&f.topic)
            ));
        }
        if let Some(g) = &f.open_group {
            let mut s = format!("Group <q>{}</q> is open", esc(g));
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
        if t.approved {
            body.push_str(&format!(
                "<p class=\"state done\">{}, approved.</p>\n",
                plural(t.count, "term", "terms")
            ));
        } else {
            body.push_str(&format!(
                "<p class=\"state waiting\">{}, not approved yet.</p>\n",
                plural(t.count, "term", "terms")
            ));
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
        if !t.entries.is_empty() {
            body.push_str("<dl class=\"entries\">\n");
            for e in &t.entries {
                body.push_str("<dt>");
                body.push_str(&esc(&e.name));
                if e.retired {
                    body.push_str(" <span class=\"retired\">(retired)</span>");
                }
                body.push_str("</dt><dd>");
                body.push_str(&esc(&e.definition));
                let aliases = if e.aliases.is_empty() {
                    "none".to_string()
                } else {
                    e.aliases.join(", ")
                };
                body.push_str(&format!(
                    "<span class=\"meta\">aliases: {} &middot; {}</span>",
                    esc(&aliases),
                    plural(e.dependents, "decision in force", "decisions in force")
                ));
                body.push_str("</dd>\n");
            }
            body.push_str("</dl>\n");
        }
        if !t.diffs.is_empty() {
            body.push_str("<div class=\"diffs\">\n<h3>Changed since approval</h3>\n<dl>\n");
            for d in &t.diffs {
                let before_aliases = if d.before_aliases.is_empty() {
                    "none".to_string()
                } else {
                    d.before_aliases.join(", ")
                };
                let after_aliases = if d.after_aliases.is_empty() {
                    "none".to_string()
                } else {
                    d.after_aliases.join(", ")
                };
                body.push_str(&format!("<dt>{}</dt><dd>\n", esc(&d.name)));
                body.push_str(&format!(
                    "<p class=\"before\">Before: {} <span class=\"meta\">aliases: {}</span></p>\n",
                    esc(&d.before_definition),
                    esc(&before_aliases)
                ));
                body.push_str(&format!(
                    "<p class=\"after\">After: {} <span class=\"meta\">aliases: {}</span></p>\n",
                    esc(&d.after_definition),
                    esc(&after_aliases)
                ));
                if !d.dependents.is_empty() {
                    body.push_str(&format!(
                        "<p class=\"meta\">Carried by: {}</p>\n",
                        esc(&d.dependents.join(", "))
                    ));
                }
                body.push_str("</dd>\n");
            }
            body.push_str("</dl>\n</div>\n");
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

    page("Overview", "/", &body)
}

fn render_next(
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
        body.push_str("<header class=\"lead\">\n<h1>Every question is answered</h1>\n");
        body.push_str("<p><a href=\"/\">Back to the overview</a></p>\n</header>\n");
        return page("Answer", "/next", &body);
    };

    let id = q.id.clone().unwrap_or_default();
    let open_ids: Vec<&str> = all
        .iter()
        .filter(|(_, x)| x.state == QState::Open)
        .filter_map(|(_, x)| x.id.as_deref())
        .collect();
    let position = open_ids.iter().position(|x| *x == id).map(|p| p + 1);

    body.push_str("<article class=\"focus\">\n");
    body.push_str("<p class=\"where\">");
    body.push_str(&format!("{}, {}", esc(&file.topic), esc(&q.group)));
    if let Some(p) = position {
        body.push_str(&format!(
            ". Question {} of {} open",
            p,
            open_ids.len()
        ));
    }
    body.push_str(".</p>\n");
    body.push_str(&format!("<h1>{}</h1>\n", esc(&q.heading)));

    if q.problem.is_some() || q.standard.is_some() {
        body.push_str("<dl class=\"framing\">\n");
        if let Some(p) = &q.problem {
            body.push_str(&format!("<dt>Problem</dt><dd>{}</dd>\n", esc(p)));
        }
        if let Some(s) = &q.standard {
            body.push_str(&format!("<dt>Standard</dt><dd>{}</dd>\n", esc(s)));
        }
        body.push_str("</dl>\n");
    }

    body.push_str("<div class=\"words\">\n<h2>Words</h2>\n");
    if q.words.is_empty() {
        body.push_str("<p class=\"meta\">No words pair an entry.</p>\n");
    } else {
        body.push_str("<dl class=\"framing\">\n");
        for w in &q.words {
            body.push_str(&format!(
                "<dt>{}</dt><dd>{} &mdash; {}</dd>\n",
                esc(&w.word),
                esc(&w.term_name),
                esc(&w.definition)
            ));
        }
        body.push_str("</dl>\n");
    }
    if !q.no_entry.is_empty() {
        body.push_str(&format!(
            "<p class=\"no-entry\">No entry: {}</p>\n",
            esc(&q.no_entry.join(", "))
        ));
    }
    body.push_str("</div>\n");

    body.push_str(&format!(
        "<form method=\"post\" action=\"/lock\" id=\"lock-form\">\n<input type=\"hidden\" name=\"id\" value=\"{}\">\n<input type=\"hidden\" name=\"t\" value=\"{}\">\n",
        esc(&id),
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
            sentence = esc(&o.sentence),
        ));
        if o.why.is_some() || o.costs.is_some() {
            body.push_str("<dl class=\"rationale\">");
            if let Some(why) = &o.why {
                body.push_str(&format!("<dt>Why</dt><dd>{}</dd>", esc(why)));
            }
            if let Some(costs) = &o.costs {
                body.push_str(&format!("<dt>Costs</dt><dd>{}</dd>", esc(costs)));
            }
            body.push_str("</dl>");
        }
        body.push_str("</span></label>\n");
    }
    body.push_str("</div>\n");
    let first = q.options.first().map(|o| o.letter).unwrap_or('A');
    body.push_str(&format!(
        "<p class=\"act\"><button type=\"submit\" class=\"lock\" id=\"lock-button\">Lock <span id=\"lock-letter\">{}</span></button> <a class=\"skip\" href=\"/next?after={}\">Skip this one</a></p>\n",
        first,
        percent_encode(&id)
    ));
    body.push_str("</form>\n");
    body.push_str("<p class=\"key-hint\">Press a letter to choose, <kbd>Enter</kbd> to lock, <kbd>S</kbd> to skip.</p>\n");
    body.push_str("</article>\n");

    body.push_str(&format!(
        r#"<script>
(function() {{
  var form = document.getElementById('lock-form');
  var letter = document.getElementById('lock-letter');
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

    page("Answer", "/next", &body)
}

fn render_decisions(snap: &Snapshot) -> String {
    let mut body = String::new();
    let total: usize = snap
        .files
        .iter()
        .map(|f| f.questions.iter().filter(|q| q.state == QState::Answered).count())
        .sum();
    body.push_str("<header class=\"lead\">\n<h1>Decisions in force</h1>\n");
    body.push_str(&format!(
        "<p class=\"summary\">{}.</p>\n</header>\n",
        plural(total, "decision", "decisions")
    ));
    for f in &snap.files {
        let answered: Vec<&QuestionView> = f
            .questions
            .iter()
            .filter(|q| q.state == QState::Answered)
            .collect();
        if answered.is_empty() {
            continue;
        }
        body.push_str("<article class=\"topic\">\n");
        body.push_str(&format!("<h2>{}</h2>\n", esc(&f.topic)));
        body.push_str("<dl class=\"decisions\">\n");
        for q in &answered {
            let sentence = chosen_sentence(q);
            let ties = match q.ties {
                0 => "no tie".to_string(),
                n => plural(n, "tie", "ties"),
            };
            body.push_str(&format!(
                "<dt>{}</dt><dd><span class=\"sentence\">{}</span><span class=\"meta\">{}",
                esc(&q.heading),
                esc(sentence),
                ties
            ));
            if !q.supersedes.is_empty() {
                body.push_str(&format!(
                    ", supersedes {}",
                    esc(&q.supersedes.join(", "))
                ));
            }
            body.push_str("</span></dd>\n");
        }
        body.push_str("</dl>\n</article>\n");
    }
    page("Decisions", "/decisions", &body)
}

fn chosen_sentence(q: &QuestionView) -> &str {
    q.options
        .iter()
        .find(|o| Some(o.letter) == q.answer)
        .map(|o| o.sentence.as_str())
        .unwrap_or("")
}

fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {}", one)
    } else {
        format!("{} {}", n, many)
    }
}

fn banner_html(m: Option<&String>) -> String {
    match m {
        Some(text) if !text.is_empty() => {
            let refused = text.contains("refusing") || text.contains("failed");
            format!(
                "<p class=\"banner{}\">{}</p>\n",
                if refused { " refused" } else { "" },
                esc(text)
            )
        }
        _ => String::new(),
    }
}

fn forbidden_page() -> String {
    simple_page(
        "Forbidden",
        "<header class=\"lead\"><h1>This link is not valid</h1><p class=\"summary\">The server prints a new link every time it starts. Use the one from the terminal that started it.</p></header>",
    )
}

fn simple_page(title: &str, body: &str) -> String {
    page(title, "", body)
}

fn page(title: &str, current: &str, body: &str) -> String {
    let nav = |href: &str, label: &str| {
        if href == current {
            format!("<a href=\"{}\" aria-current=\"page\">{}</a>", href, label)
        } else {
            format!("<a href=\"{}\">{}</a>", href, label)
        }
    };
    format!(
        "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{title}</title><style>{style}</style></head><body>\n<nav class=\"bar\"><span class=\"mark\">adr</span>{home}{next}{decisions}</nav>\n<main>\n{body}</main>\n</body></html>\n",
        title = esc(title),
        style = STYLE,
        home = nav("/", "Overview"),
        next = nav("/next", "Answer"),
        decisions = nav("/decisions", "Decisions"),
        body = body
    )
}

const STYLE: &str = r#"
:root {
  color-scheme: light dark;
  --paper: #f3f4f1;
  --ink: #17191c;
  --muted: #5f6570;
  --rule: #cfd3d6;
  --hold: #1e6b47;
  --hold-soft: rgba(30, 107, 71, 0.10);
  --refuse: #a32a22;
  --refuse-soft: rgba(163, 42, 34, 0.08);
  --key: #ffffff;
}
@media (prefers-color-scheme: dark) {
  :root {
    --paper: #15171a;
    --ink: #e6e8ea;
    --muted: #9aa1aa;
    --rule: #33383e;
    --hold: #5cc394;
    --hold-soft: rgba(92, 195, 148, 0.12);
    --refuse: #e0665c;
    --refuse-soft: rgba(224, 102, 92, 0.12);
    --key: #1f2226;
  }
}
* { box-sizing: border-box; }
html { background: var(--paper); color: var(--ink); }
body {
  margin: 0;
  font-family: "Bitstream Charter", Charter, "Iowan Old Style", P052, "Palatino Linotype", Palatino, "Noto Serif", Georgia, serif;
  font-size: 19px;
  line-height: 1.55;
}
main { max-width: 42rem; margin: 0 auto; padding: 1.5rem 1rem 5rem; }
a { color: inherit; text-decoration-color: var(--rule); text-decoration-thickness: 1px; text-underline-offset: 0.15em; }
a:hover { text-decoration-color: currentColor; }
q { quotes: "\201C" "\201D"; }

.bar {
  display: flex; align-items: baseline; gap: 1.4rem;
  max-width: 42rem; margin: 0 auto; padding: 1rem 1rem 0;
  font-size: 0.95rem; color: var(--muted);
}
.bar .mark { font-weight: 700; color: var(--ink); letter-spacing: 0.02em; margin-right: auto; }
.bar a { text-decoration: none; }
.bar a[aria-current] { color: var(--ink); border-bottom: 2px solid var(--hold); }

h1 { font-size: 2.25rem; line-height: 1.15; font-weight: 700; margin: 1.6rem 0 0.6rem; letter-spacing: -0.01em; }
h2 { font-size: 1.35rem; line-height: 1.25; font-weight: 700; margin: 0 0 0.2rem; }
.lead .summary { font-size: 1.15rem; color: var(--muted); margin: 0 0 1rem; }
.primary {
  display: inline-block; text-decoration: none;
  background: var(--hold); color: var(--paper);
  padding: 0.55em 1.1em; border-radius: 4px; font-weight: 700;
}
.primary:hover { filter: brightness(1.08); }

.topic { border-top: 1px solid var(--rule); padding: 1.4rem 0 1.1rem; }
.topic .path { margin: 0 0 0.6rem; font-size: 0.85rem; color: var(--muted); }
.state { margin: 0.3rem 0; }
.state.waiting { color: var(--muted); }
.state.done { color: var(--hold); }
.state.failing { color: var(--refuse); }
.inline { margin: 0.5rem 0 0.8rem; }
.open-list { margin: 0.6rem 0 0.4rem; padding-left: 1.4rem; }
.open-list li { margin: 0.2rem 0; }
.open-list a { text-decoration: none; }
.open-list a:hover { text-decoration: underline; }
details.answered { margin-top: 0.6rem; color: var(--muted); }
details summary { cursor: pointer; }
details.answered dl { margin: 0.5rem 0 0; }
details.answered dt { color: var(--muted); font-size: 0.95rem; }
details.answered dd { margin: 0 0 0.6rem; color: var(--ink); }

.failures { color: var(--refuse); padding-left: 1.2rem; margin: 0.4rem 0; }
.failures li { margin: 0.15rem 0; }
.notes { color: var(--muted); padding-left: 1.2rem; margin: 0.4rem 0; font-size: 0.95rem; }
details.unban { margin-top: 2rem; color: var(--muted); font-size: 0.95rem; }
details.unban form { margin-top: 0.6rem; }

.banner {
  margin: 1.2rem 0 0; padding: 0.7em 1em;
  border-left: 4px solid var(--hold); background: var(--hold-soft);
}
.banner.refused { border-left-color: var(--refuse); background: var(--refuse-soft); }

.focus .where { margin: 1.4rem 0 0; color: var(--muted); font-size: 0.95rem; }
.focus h1 { font-size: 2.6rem; margin-top: 0.3rem; }
dl.framing { margin: 1.2rem 0 1.8rem; display: grid; grid-template-columns: 6rem 1fr; column-gap: 1rem; row-gap: 0.5rem; }
dl.framing dt { color: var(--muted); }
dl.framing dd { margin: 0; }

.options { display: flex; flex-direction: column; margin: 0 0 1.4rem; border-top: 1px solid var(--rule); }
.option {
  display: grid; grid-template-columns: 3.2rem 1fr; column-gap: 1rem;
  padding: 1rem 0.6rem 1rem 0.6rem; border-bottom: 1px solid var(--rule);
  cursor: pointer; position: relative;
}
.option input { position: absolute; opacity: 0; width: 0; height: 0; }
.option .key {
  align-self: start; justify-self: start;
  width: 2.6rem; height: 2.6rem; display: grid; place-items: center;
  font-weight: 700; font-size: 1.25rem;
  background: var(--key); border: 1px solid var(--rule); border-bottom-width: 3px; border-radius: 6px;
}
.option .text { display: flex; flex-direction: column; gap: 0.45rem; }
.option .sentence { font-size: 1.12rem; line-height: 1.45; }
dl.rationale { margin: 0; display: grid; grid-template-columns: 3.6rem 1fr; column-gap: 0.8rem; row-gap: 0.2rem; font-size: 0.97rem; color: var(--muted); }
dl.rationale dt { }
dl.rationale dd { margin: 0; }
.option:has(input:checked) { box-shadow: inset 4px 0 0 var(--hold); background: var(--hold-soft); }
.option:has(input:checked) .key { background: var(--hold); color: var(--paper); border-color: var(--hold); }
.option:has(input:focus-visible) { outline: 2px solid var(--hold); outline-offset: 2px; }

.act { display: flex; align-items: center; gap: 1.2rem; margin: 0 0 0.6rem; }
button {
  font: inherit; font-weight: 700; font-size: 1.05rem;
  padding: 0.55em 1.2em; border-radius: 4px; cursor: pointer;
  background: var(--key); color: var(--ink); border: 1px solid var(--muted);
}
button.lock { background: var(--hold); color: var(--paper); border-color: var(--hold); min-width: 7.5rem; }
button:hover { filter: brightness(1.08); }
button:focus-visible { outline: 2px solid var(--hold); outline-offset: 2px; }
.skip { color: var(--muted); }
.key-hint { color: var(--muted); font-size: 0.92rem; }
kbd { font-family: inherit; font-size: 0.9em; padding: 0.05em 0.45em; border: 1px solid var(--rule); border-bottom-width: 2px; border-radius: 4px; background: var(--key); }

dl.decisions { margin: 0.6rem 0 0; }
dl.decisions dt { color: var(--muted); margin-top: 0.8rem; }
dl.decisions dd { margin: 0.1rem 0 0; }
dl.decisions .sentence { display: block; font-size: 1.08rem; }
dl.decisions .meta { display: block; color: var(--muted); font-size: 0.9rem; }

.words { margin: 0 0 1.4rem; }
.words h2 { font-size: 1.1rem; }
.no-entry { color: var(--muted); font-size: 0.92rem; margin: 0.4rem 0 0; }

dl.entries { margin: 0.6rem 0 0; }
dl.entries dt { color: var(--ink); font-weight: 700; margin-top: 0.8rem; }
dl.entries dd { margin: 0.1rem 0 0; }
dl.entries .meta { display: block; color: var(--muted); font-size: 0.9rem; }
.retired { color: var(--refuse); font-weight: 400; font-size: 0.85em; }

.diffs { margin-top: 1rem; }
.diffs dt { font-weight: 700; margin-top: 0.8rem; }
.diffs .before, .diffs .after { margin: 0.2rem 0; }
.diffs .meta { display: block; color: var(--muted); font-size: 0.9rem; }

textarea { width: 100%; font: inherit; font-size: 0.95rem; padding: 0.5em; background: var(--key); color: var(--ink); border: 1px solid var(--rule); border-radius: 4px; }

@media (max-width: 600px) {
  body { font-size: 17px; }
  h1, .focus h1 { font-size: 1.9rem; }
  dl.framing { grid-template-columns: 1fr; row-gap: 0.1rem; }
  dl.framing dt { margin-top: 0.5rem; }
  .option { grid-template-columns: 2.6rem 1fr; padding-left: 0.2rem; padding-right: 0.2rem; }
  .option .key { width: 2.2rem; height: 2.2rem; font-size: 1.05rem; }
}
@media (prefers-reduced-motion: no-preference) {
  .option, .primary, button { transition: background-color 120ms ease, box-shadow 120ms ease; }
}
"#;

// --- view helpers ------------------------------------------------------

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
        if let Some(pos) = all.iter().position(|(_, q)| q.id.as_deref() == Some(after_id)) {
            return all[pos + 1..]
                .iter()
                .find(|(_, q)| q.state == QState::Open)
                .copied();
        }
    }
    all.iter().find(|(_, q)| q.state == QState::Open).copied()
}

// --- small hand-rolled parsers/encoders (no extra crates) ---------------

fn split_path_query(url: &str) -> (String, String) {
    match url.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (url.to_string(), String::new()),
    }
}

fn strip_query_key(path: &str, query: &str, key: &str) -> String {
    let kept: Vec<&str> = query
        .split('&')
        .filter(|p| {
            if p.is_empty() {
                return false;
            }
            let k = p.split('=').next().unwrap_or("");
            percent_decode(k) != key
        })
        .collect();
    if kept.is_empty() {
        path.to_string()
    } else {
        format!("{}?{}", path, kept.join("&"))
    }
}

fn parse_form(s: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if s.is_empty() {
        return map;
    }
    for pair in s.split('&') {
        if pair.is_empty() {
            continue;
        }
        let mut it = pair.splitn(2, '=');
        let k = it.next().unwrap_or("");
        let v = it.next().unwrap_or("");
        map.insert(percent_decode(k), percent_decode(v));
    }
    map
}

/// Constant-time equality for the owner token: every byte is compared
/// regardless of where the first mismatch is, so response time does not
/// leak how many leading bytes a guess got right.
fn token_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len()
                && bytes[i + 1].is_ascii_hexdigit()
                && bytes[i + 2].is_ascii_hexdigit() =>
            {
                let hi = (bytes[i + 1] as char).to_digit(16).unwrap_or(0) as u8;
                let lo = (bytes[i + 2] as char).to_digit(16).unwrap_or(0) as u8;
                out.push(hi * 16 + lo);
                i += 3;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn cookie_value(headers: &[Header], name: &str) -> Option<String> {
    for h in headers {
        if h.field.equiv("Cookie") {
            let val = h.value.as_str().to_string();
            for part in val.split(';') {
                let part = part.trim();
                if let Some((k, v)) = part.split_once('=') {
                    if k == name {
                        return Some(v.to_string());
                    }
                }
            }
        }
    }
    None
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{percent_decode, token_eq};

    #[test]
    fn percent_decode_never_slices_inside_a_multibyte_char() {
        // "%" directly followed by a three-byte character: the old code
        // sliced the &str at byte 3, inside the character, and panicked.
        assert_eq!(percent_decode("%€"), "%€");
        assert_eq!(percent_decode("a%2Fb+c"), "a/b c");
        assert_eq!(percent_decode("%zz"), "%zz");
        assert_eq!(percent_decode("%2"), "%2");
    }

    #[test]
    fn token_eq_compares_whole_strings() {
        assert!(token_eq("abc", "abc"));
        assert!(!token_eq("abc", "abd"));
        assert!(!token_eq("abc", "ab"));
        assert!(!token_eq("", "a"));
    }
}
