//! The owner's page: served by the binary on loopback, token-gated.
mod answer;
mod decisions;
mod html;
mod overview;
mod terms;

use crate::actions;
use crate::msg::{self, Actor};
use crate::repo;
use crate::view;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use tiny_http::{Header, Method, Request, Response, Server};

pub fn run(port: u16) -> i32 {
    if !repo::owner_authorized() {
        eprintln!("{}", msg::owner_refusal("serve", ""));
        return 2;
    }

    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("{}", msg::not_a_repo("serve", &format!("--port {}", port), Actor::Owner));
            return 1;
        }
    };

    let server = match Server::http(("127.0.0.1", port)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("serve: {}", msg::bind_failed(port, e));
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
        send(request, Reply::Html(403, html::forbidden_page()));
        return;
    }
    if method == Method::Post && !form.get("t").is_some_and(|t| token_eq(t, token)) {
        send(request, Reply::Html(403, html::forbidden_page()));
        return;
    }

    let reply = match (method, path.as_str()) {
        (Method::Get, "/") => {
            let snap = view::snapshot(root);
            Reply::Html(200, overview::render_index(&snap, query.get("m"), token))
        }
        (Method::Get, "/next") => {
            let snap = view::snapshot(root);
            Reply::Html(
                200,
                answer::render_next(
                    &snap,
                    query.get("after"),
                    query.get("show"),
                    query.get("m"),
                    token,
                ),
            )
        }
        (Method::Get, "/decisions") => {
            let snap = view::snapshot(root);
            Reply::Html(200, decisions::render_decisions(&snap))
        }
        (Method::Get, "/terms") => {
            let snap = view::snapshot(root);
            Reply::Html(200, terms::render_terms(&snap, query.get("m"), token))
        }
        (Method::Post, "/approve") => handle_approve(root, &form),
        (Method::Post, "/answer") => handle_answer(root, &form),
        (Method::Post, "/unban") => handle_unban(root, &form),
        _ => Reply::Html(404, html::simple_page("Not found", "<p>Not found.</p>")),
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
        Ok(o) => o.summary(),
        Err(e) => e,
    };
    Reply::Redirect(format!("/?m={}", percent_encode(&banner)), None)
}

fn handle_answer(root: &Path, form: &HashMap<String, String>) -> Reply {
    let id = form.get("id").cloned().unwrap_or_default();
    // Per "Does an answer carry the version of the question the owner
    // read?": the page's hidden `v` field carries the version shown on the
    // form it displayed; a missing field (never happens from this page's
    // own form, only from a hand-crafted POST) never matches a real
    // section's hash and so refuses the same way a stale one does.
    let version = form.get("v").cloned().unwrap_or_default();
    let value = form.get("answer").map(|s| s.trim().to_string()).unwrap_or_default();

    // The Refuse button posts `answer=refuse`; everything else is a letter.
    let outcome = if value == "refuse" {
        Some(actions::do_refuse(root, &id, &version))
    } else {
        None
    };
    if let Some(result) = outcome {
        return match result {
            Ok(o) => Reply::Redirect(
                format!("/next?after={}&m={}", percent_encode(&id), percent_encode(&o.summary())),
                None,
            ),
            Err(e) => Reply::Redirect(
                format!("/next?show={}&m={}", percent_encode(&id), percent_encode(&e)),
                None,
            ),
        };
    }

    // Same letter parsing the CLI uses (`actions::parse_letter`), so a
    // lowercase or multi-character selection refuses the same way here as
    // it does from the terminal; an empty selection (no field posted at
    // all -- never happens from this page's own form) keeps its own
    // message, since there is nothing malformed to name.
    let Some(letter) = actions::parse_letter(&value) else {
        let banner = if value.is_empty() {
            format!("serve: {}", msg::missing_answer_selection(&id, &version))
        } else {
            format!("serve: {}", msg::invalid_letter(&value, &format!("{}@{}", id, version)))
        };
        return Reply::Redirect(
            format!(
                "/next?show={}&m={}",
                percent_encode(&id),
                percent_encode(&banner)
            ),
            None,
        );
    };

    match actions::do_answer(root, &id, &version, letter) {
        Ok(o) => Reply::Redirect(
            format!(
                "/next?after={}&m={}",
                percent_encode(&id),
                percent_encode(&o.summary())
            ),
            None,
        ),
        Err(e) => Reply::Redirect(
            format!(
                "/next?show={}&m={}",
                percent_encode(&id),
                percent_encode(&e)
            ),
            None,
        ),
    }
}

fn handle_unban(root: &Path, form: &HashMap<String, String>) -> Reply {
    let lines = form.get("lines").cloned().unwrap_or_default();
    let banner = match actions::do_unban(root, &lines) {
        Ok(o) => o.summary(),
        Err(e) => e,
    };
    Reply::Redirect(format!("/?m={}", percent_encode(&banner)), None)
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
