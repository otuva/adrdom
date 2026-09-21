use super::Failure;
use crate::record::RecordFile;

pub(super) fn check_parse(records: &[RecordFile]) -> Vec<Failure> {
    let mut out = Vec::new();
    for rf in records {
        for e in &rf.shape_errors {
            out.push(Failure {
                check: "parse".to_string(),
                id: None,
                message: format!("{}: {}", rf.rel, e),
            });
        }
        for sec in &rf.sections {
            for e in &sec.parse_errors {
                out.push(Failure {
                    check: "parse".to_string(),
                    id: sec.id.clone(),
                    message: format!("{}: {}: {}", rf.rel, sec.heading_text, e),
                });
            }
            for e in &sec.shape_errors {
                out.push(Failure {
                    check: "parse".to_string(),
                    id: sec.id.clone(),
                    message: format!("{}: {}: {}", rf.rel, sec.heading_text, e),
                });
            }
        }
    }
    out
}
