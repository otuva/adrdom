use std::fs;
use std::io::Write;
use std::path::Path;

pub const EVENTS: [&str; 5] = ["asked", "answered", "superseded", "overridden", "banned"];

#[derive(Debug, Clone)]
pub struct Row {
    pub time: String,
    pub event: String,
    pub id: String,
    pub hash: String,
    pub detail: String,
}

pub struct Ledger {
    pub rows: Vec<Row>,
    pub malformed: Vec<(usize, String)>,
}

pub fn now_rfc3339() -> String {
    use time::format_description::well_known::Rfc3339;
    use time::OffsetDateTime;
    let now = OffsetDateTime::now_utc().replace_nanosecond(0).unwrap();
    now.format(&Rfc3339).unwrap()
}

pub fn read(path: &Path) -> Ledger {
    let content = fs::read_to_string(path).unwrap_or_default();
    let mut rows = Vec::new();
    let mut malformed = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() != 5 || !EVENTS.contains(&parts[1]) {
            malformed.push((idx + 1, line.to_string()));
            continue;
        }
        rows.push(Row {
            time: parts[0].to_string(),
            event: parts[1].to_string(),
            id: parts[2].to_string(),
            hash: parts[3].to_string(),
            detail: parts[4].to_string(),
        });
    }
    Ledger { rows, malformed }
}

pub fn row_line(r: &Row) -> String {
    let detail = r.detail.replace(['\t', '\n'], " ");
    format!("{}\t{}\t{}\t{}\t{}", r.time, r.event, r.id, r.hash, detail)
}

pub fn append(path: &Path, rows: &[Row]) -> std::io::Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    for r in rows {
        writeln!(f, "{}", row_line(r))?;
    }
    Ok(())
}

impl Ledger {
    pub fn latest_state(&self, id: &str) -> Option<&Row> {
        self.rows.iter().rfind(|r| {
            r.id == id && matches!(r.event.as_str(), "asked" | "answered" | "superseded")
        })
    }

    pub fn latest_asked(&self, id: &str) -> Option<&Row> {
        self.rows.iter().rfind(|r| r.id == id && r.event == "asked")
    }

    pub fn overrides_for(&self, check: &str) -> Vec<&Row> {
        let prefix = format!("{}: ", check);
        self.rows
            .iter()
            .filter(|r| r.event == "overridden" && r.detail.starts_with(&prefix))
            .collect()
    }
}
