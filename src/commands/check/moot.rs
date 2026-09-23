use crate::record::{AnswerSet, Dep, RecordFile};

/// `moot` (8.7 A): a note, never a failure -- one line for every section
/// whose heading `depends:` entry carries an answer set that excludes the
/// letter the target question was actually answered with. The plan still
/// asks the agent to look, but it is not wrong on its own: the owner may
/// simply not have re-planned yet.
pub(super) fn check_moot(records: &[RecordFile]) -> Vec<String> {
    let mut out = Vec::new();
    for rf in records {
        for sec in &rf.sections {
            for d in &sec.depends {
                let Dep::Heading {
                    text,
                    set: Some(AnswerSet::Letters(letters)),
                } = d
                else {
                    continue;
                };
                let target_text = text.trim();
                let Some(target) = rf.sections.iter().find(|s| s.heading_text.trim() == target_text) else {
                    continue;
                };
                let Some(answered) = target.answer_letter() else {
                    continue;
                };
                if letters.contains(&answered) {
                    continue;
                }
                let set_str = letters.iter().map(|c| c.to_string()).collect::<Vec<_>>().join("|");
                out.push(format!(
                    "moot: {}: \"{}\" stands on \"{}\"={}, answered {}",
                    rf.rel, sec.heading_text, target_text, set_str, answered
                ));
            }
        }
    }
    out
}
