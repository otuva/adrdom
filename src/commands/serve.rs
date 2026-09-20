//! The owner's page (7.x): served by the binary on loopback, token-gated.
use crate::actions;
use crate::repo;
use crate::view::{self, QState, QuestionView, Snapshot};
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
            if t == token {
                let dest = strip_query_key(&path, &query_string, "t");
                send(
                    request,
                    Reply::Redirect(dest, Some(cookie_header_value(token))),
                );
                return;
            }
        }
    }

    let cookie_ok = cookie_token.as_deref() == Some(token);
    if !cookie_ok {
        send(request, Reply::Html(403, forbidden_page()));
        return;
    }
    if method == Method::Post && form.get("t").map(String::as_str) != Some(token) {
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

    body.push_str("<h1>adrdom</h1>\n");
    body.push_str(&format!(
        "<p><a class=\"next-link\" href=\"/next\">Answer next &rarr;</a> &middot; <a href=\"/decisions\">Decisions</a> &middot; {} open</p>\n",
        view::open_questions(snap).len()
    ));

    for f in &snap.files {
        body.push_str("<section class=\"file\">\n");
        body.push_str(&format!(
            "<h2>{} <span class=\"path\">{}</span></h2>\n",
            esc(&f.topic),
            esc(&f.path)
        ));

        let mut status_parts: Vec<String> = Vec::new();
        let mut awaiting: Vec<&str> = Vec::new();
        if !f.plan_approved {
            awaiting.push("plan");
        }
        if !f.context_approved {
            awaiting.push("context");
        }
        if !awaiting.is_empty() {
            status_parts.push(format!("awaiting approval: {}", awaiting.join("/")));
        }
        if let Some(g) = &f.open_group {
            let mut s = format!("open group {}", esc(g));
            if let Some(w) = &f.waits_on {
                s.push_str(&format!(" &mdash; waits on \"{}\"", esc(w)));
            }
            status_parts.push(s);
        }
        if f.done {
            status_parts.push("done".to_string());
        }
        if !status_parts.is_empty() {
            body.push_str(&format!(
                "<p class=\"status\">{}</p>\n",
                status_parts.join(" &middot; ")
            ));
        }

        if !awaiting.is_empty() {
            body.push_str(&format!(
                "<form method=\"post\" action=\"/approve\"><input type=\"hidden\" name=\"file\" value=\"{}\"><input type=\"hidden\" name=\"t\" value=\"{}\"><button type=\"submit\">Approve</button></form>\n",
                esc(&f.path),
                esc(token)
            ));
        }

        let open: Vec<&QuestionView> = f
            .questions
            .iter()
            .filter(|q| q.state == QState::Open)
            .collect();
        if !open.is_empty() {
            body.push_str("<ul class=\"open-list\">\n");
            for q in &open {
                let id = q.id.clone().unwrap_or_default();
                body.push_str(&format!(
                    "<li><a href=\"/next?show={}\">{}</a></li>\n",
                    percent_encode(&id),
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
            body.push_str("<details><summary>Answered</summary>\n<ul>\n");
            for q in &answered {
                let sentence = q
                    .options
                    .iter()
                    .find(|o| Some(o.letter) == q.answer)
                    .map(|o| o.sentence.as_str())
                    .unwrap_or("");
                body.push_str(&format!(
                    "<li>{} &mdash; {}</li>\n",
                    esc(&q.heading),
                    esc(sentence)
                ));
            }
            body.push_str("</ul></details>\n");
        }

        body.push_str("</section>\n");
    }

    if let Some(t) = &snap.terms {
        body.push_str("<section class=\"terms\">\n<h2>Terms</h2>\n");
        body.push_str(&format!("<p>{}</p>\n", esc(&t.path)));
        if t.approved {
            body.push_str("<p class=\"status\">approved</p>\n");
        } else {
            body.push_str(&format!(
                "<form method=\"post\" action=\"/approve\"><input type=\"hidden\" name=\"file\" value=\"{}\"><input type=\"hidden\" name=\"t\" value=\"{}\"><button type=\"submit\">Approve</button></form>\n",
                esc(&t.path),
                esc(token)
            ));
        }
        body.push_str(&format!("<p>{} term(s)</p>\n", t.count));
        if !t.errors.is_empty() {
            body.push_str("<ul class=\"failures\">\n");
            for e in &t.errors {
                body.push_str(&format!("<li>{}</li>\n", esc(e)));
            }
            body.push_str("</ul>\n");
        }
        body.push_str("</section>\n");
    }

    body.push_str("<section class=\"check\">\n<h2>Check</h2>\n");
    if !snap.check.failures.is_empty() {
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
    body.push_str("</section>\n");

    body.push_str("<section class=\"unban\">\n<h2>Unban</h2>\n");
    body.push_str(&format!(
        "<form method=\"post\" action=\"/unban\"><textarea name=\"lines\" rows=\"4\" placeholder=\"one line per phrase to unban\"></textarea><input type=\"hidden\" name=\"t\" value=\"{}\"><button type=\"submit\">Unban</button></form>\n",
        esc(token)
    ));
    body.push_str("</section>\n");

    page("adrdom", &body)
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
        body.push_str("<h1>All decisions answered</h1>\n");
        body.push_str("<p><a href=\"/\">Back</a></p>\n");
        return page("Next", &body);
    };

    let id = q.id.clone().unwrap_or_default();
    body.push_str("<div class=\"focus\">\n");
    body.push_str(&format!(
        "<p class=\"where\">{} &middot; {}</p>\n",
        esc(file),
        esc(&q.group)
    ));
    body.push_str(&format!("<h1>{}</h1>\n", esc(&q.heading)));
    if let Some(p) = &q.problem {
        body.push_str(&format!("<p class=\"problem\"><strong>Problem:</strong> {}</p>\n", esc(p)));
    }
    if let Some(s) = &q.standard {
        body.push_str(&format!("<p class=\"standard\"><strong>Standard:</strong> {}</p>\n", esc(s)));
    }

    body.push_str(&format!(
        "<form method=\"post\" action=\"/lock\" id=\"lock-form\">\n<input type=\"hidden\" name=\"id\" value=\"{}\">\n<input type=\"hidden\" name=\"t\" value=\"{}\">\n",
        esc(&id),
        esc(token)
    ));
    body.push_str("<div class=\"options\">\n");
    for (i, o) in q.options.iter().enumerate() {
        let input_id = format!("opt-{}", o.letter);
        body.push_str(&format!(
            "<label class=\"card\" for=\"{iid}\"><input type=\"radio\" name=\"answer\" id=\"{iid}\" value=\"{letter}\"{checked}><span class=\"letter\">{letter}</span><span class=\"sentence\">{sentence}</span>",
            iid = input_id,
            letter = o.letter,
            checked = if i == 0 { " checked" } else { "" },
            sentence = esc(&o.sentence),
        ));
        if let Some(why) = &o.why {
            body.push_str(&format!("<span class=\"why\">Why: {}</span>", esc(why)));
        }
        if let Some(costs) = &o.costs {
            body.push_str(&format!("<span class=\"costs\">Costs: {}</span>", esc(costs)));
        }
        body.push_str("</label>\n");
    }
    body.push_str("</div>\n");
    body.push_str("<button type=\"submit\">Lock</button>\n");
    body.push_str("</form>\n");
    body.push_str(&format!(
        "<p class=\"skip\"><a href=\"/next?after={}\">Skip &rarr;</a></p>\n",
        percent_encode(&id)
    ));
    body.push_str("<p class=\"key-hint\"><kbd>A</kbd>&ndash;<kbd>E</kbd> pick an option, <kbd>Enter</kbd> submits, <kbd>S</kbd> skips.</p>\n");
    body.push_str("</div>\n");

    body.push_str(&format!(
        r#"<script>
(function() {{
  var form = document.getElementById('lock-form');
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
      if (radio) {{ radio.checked = true; }}
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

    page("Next", &body)
}

fn render_decisions(snap: &Snapshot) -> String {
    let mut body = String::new();
    body.push_str("<h1>Decisions</h1>\n<ul class=\"decisions\">\n");
    for f in &snap.files {
        for q in &f.questions {
            if q.state != QState::Answered {
                continue;
            }
            let sentence = q
                .options
                .iter()
                .find(|o| Some(o.letter) == q.answer)
                .map(|o| o.sentence.as_str())
                .unwrap_or("");
            let supersedes = if q.supersedes.is_empty() {
                String::new()
            } else {
                format!(
                    " <span class=\"supersedes\">(supersedes {})</span>",
                    esc(&q.supersedes.join(", "))
                )
            };
            body.push_str(&format!(
                "<li><strong>{}</strong> &mdash; {} <span class=\"ties\">({} tie(s))</span>{}</li>\n",
                esc(&q.heading),
                esc(sentence),
                q.ties,
                supersedes
            ));
        }
    }
    body.push_str("</ul>\n<p><a href=\"/\">Back</a></p>\n");
    page("Decisions", &body)
}

fn banner_html(m: Option<&String>) -> String {
    match m {
        Some(text) if !text.is_empty() => {
            format!("<p class=\"banner\">{}</p>\n", esc(text))
        }
        _ => String::new(),
    }
}

fn forbidden_page() -> String {
    simple_page(
        "Forbidden",
        "<p>This link is not valid. The server prints a new link every time it starts.</p>",
    )
}

fn simple_page(title: &str, body: &str) -> String {
    page(title, body)
}

fn page(title: &str, body: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><title>{}</title><style>{}</style></head><body>{}</body></html>\n",
        esc(title),
        STYLE,
        body
    )
}

const STYLE: &str = r#"
:root { color-scheme: light dark; }
body { font: 18px/1.5 -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; max-width: 46rem; margin: 2rem auto; padding: 0 1rem; }
h1 { font-size: 2rem; }
.path { color: #777; font-size: 0.8em; font-weight: normal; }
.status { color: #555; }
.banner { background: #eef; border: 1px solid #99c; padding: 0.6em 1em; border-radius: 6px; }
.failures { color: #a00; }
.notes { color: #555; }
section { margin: 2rem 0; padding-bottom: 1rem; border-bottom: 1px solid #ddd; }
.focus h1 { font-size: 2.4rem; }
.focus .problem, .focus .standard { font-size: 1.1rem; }
.options { display: flex; flex-direction: column; gap: 0.75rem; margin: 1rem 0; }
.card { display: flex; flex-direction: column; gap: 0.2rem; border: 2px solid #ccc; border-radius: 10px; padding: 1rem; cursor: pointer; }
.card:has(input:checked) { border-color: #36c; background: rgba(51, 102, 204, 0.08); }
.card .letter { font-weight: bold; font-size: 1.2rem; }
.card .why, .card .costs { color: #555; font-size: 0.95rem; }
.card input { margin-right: 0.5rem; }
button { font-size: 1.1rem; padding: 0.5em 1.2em; border-radius: 6px; border: 1px solid #888; cursor: pointer; }
kbd { border: 1px solid #999; border-radius: 4px; padding: 0.1em 0.4em; font-family: monospace; background: #f4f4f4; }
textarea { width: 100%; font: inherit; }
.open-list a { text-decoration: none; }
.next-link { font-weight: bold; }
"#;

// --- view helpers ------------------------------------------------------

fn full_question_list(snap: &Snapshot) -> Vec<(&str, &QuestionView)> {
    let mut out = Vec::new();
    for f in &snap.files {
        for q in &f.questions {
            out.push((f.path.as_str(), q));
        }
    }
    out
}

fn pick_next<'a>(
    all: &[(&'a str, &'a QuestionView)],
    after: Option<&str>,
    show: Option<&str>,
) -> Option<(&'a str, &'a QuestionView)> {
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
            b'%' if i + 2 < bytes.len() => {
                if let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    out.push(byte);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
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
