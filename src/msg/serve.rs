//! The messages of `serve`.

use super::*;

// --- serve/mod.rs ------------------------------------------------------

pub fn bind_failed(port: u16, e: impl Display) -> String {
    refuse(
        format!("failed to bind 127.0.0.1:{}: {}", port, e),
        Next::owner_runs(Cmd::fixed("adr serve --port").then("<port>")),
    )
}

pub fn missing_answer_selection(id: &str, version: &str) -> String {
    refuse(
        "missing answer selection".to_string(),
        crate::moves::clause(MoveKind::Answer {
            id: id.to_string(),
            version: version.to_string(),
            heading: String::new(),
        }),
    )
}
