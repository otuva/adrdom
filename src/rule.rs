//! Every rule a finding can report: one constant each, with the decisions the
//! rule comes from. A finding prints `<found>; <move>`, nothing on what the
//! rule protects (per "What does a finding line hold?", 50b31713 A).
//! A rule with no decision is open: no current decision states it yet.

/// A rule the records can break.
#[derive(Debug, PartialEq, Eq)]
pub struct Rule {
    pub name: &'static str,
    /// The identifiers of the current decisions the rule comes from.
    pub decisions: &'static [&'static str],
}

/// Decision: What is the shape of a topic file?
pub const TOPIC_FILE_SHAPE: Rule = Rule {
    name: "topic_file_shape",
    decisions: &["9913568f-6b85-4d26-946b-c6e108f9aa51"],
};

/// Decision: What are the tiers derived from?
pub const TIER_HEADING_ABSENT: Rule = Rule {
    name: "tier_heading_absent",
    decisions: &["f1b4c4cc-97bd-446f-87b4-df5f1b6f4da2"],
};

/// Decision: What is the shape of a topic file?
pub const QUESTION_HEADING_ENDS_IN_MARK: Rule = Rule {
    name: "question_heading_ends_in_mark",
    decisions: &["9913568f-6b85-4d26-946b-c6e108f9aa51"],
};

/// Decision: What stands under a question's heading?
pub const QUESTION_SECTION_SHAPE: Rule = Rule {
    name: "question_section_shape",
    decisions: &["178a232e-b51c-4122-a1a9-9e9ed6970748"],
};

/// Decision: Does a reference line keep its list on one line?
pub const REFERENCE_LINE_ONCE: Rule = Rule {
    name: "reference_line_once",
    decisions: &["b0554224-15e4-4e91-a2bb-9ae5667803f4"],
};

/// Decision: What stands under a question's heading?
pub const OPTION_IS_ONE_SENTENCE: Rule = Rule {
    name: "option_is_one_sentence",
    decisions: &["178a232e-b51c-4122-a1a9-9e9ed6970748"],
};

/// Decision: Does a question keep its identifier when its heading is reworded?
pub const ID_ASSIGNED_BY_TOOL: Rule = Rule {
    name: "id_assigned_by_tool",
    decisions: &["7a1e7b81-0967-4ed2-8a8a-c7153056836f"],
};

/// Decision: Does an option under a heading that opens with Does hold one word?
pub const DOES_OPTION_IS_ONE_WORD: Rule = Rule {
    name: "does_option_is_one_word",
    decisions: &["b3cb190d-d6f1-4930-bdcd-7827e31ac62a"],
};

/// Decision: What is the word limit of an option?
pub const OPTION_WORD_LIMIT: Rule = Rule {
    name: "option_word_limit",
    decisions: &["8f324216-80a3-4012-b689-e27b6eea7714"],
};

/// Decision: Does the word limit hold for an option's Why line?
pub const WHY_WORD_LIMIT: Rule = Rule {
    name: "why_word_limit",
    decisions: &["0487a1cf-eb61-4bf8-bd16-eed899d22d2c"],
};

/// Decision: Does the word limit hold for an option's Costs line?
pub const COSTS_WORD_LIMIT: Rule = Rule {
    name: "costs_word_limit",
    decisions: &["c96518bf-470d-45c0-8b86-eaf8f54cb490"],
};

/// Decision: What stops a question that has no depends line?
pub const QUESTION_HAS_DEPENDS_LINE: Rule = Rule {
    name: "question_has_depends_line",
    decisions: &["9bf76db8-4821-4257-ac49-4c22c2a0e538"],
};

/// Decision: What stops a depends entry that resolves to nothing?
pub const DEPENDS_ENTRY_RESOLVES: Rule = Rule {
    name: "depends_entry_resolves",
    decisions: &["49a1c3a8-07ea-448a-8926-2362c0617758"],
};

/// Decision: What does a depends entry name?
pub const DEPENDS_ENTRY_NAMES_TERM_OR_FILE_QUESTION: Rule = Rule {
    name: "depends_entry_names_term_or_file_question",
    decisions: &["9afe335c-2a7a-4625-b115-f74d3f091cac"],
};

/// Decision: What stops a depends entry that resolves to nothing?
pub const DEPENDS_TERM_IS_LIVE: Rule = Rule {
    name: "depends_term_is_live",
    decisions: &["49a1c3a8-07ea-448a-8926-2362c0617758"],
};

/// Decision: What stops a depends entry that resolves to nothing?
pub const DEPENDS_TERM_EXISTS: Rule = Rule {
    name: "depends_term_exists",
    decisions: &["49a1c3a8-07ea-448a-8926-2362c0617758"],
};

/// Decision: Does a decision change after it is answered?
pub const ANSWERED_DEPENDS_LINE_FROZEN: Rule = Rule {
    name: "answered_depends_line_frozen",
    decisions: &["dad5273c-afe2-4ab1-a451-e2a5d4d9a26d"],
};

/// Decision: Does a depends entry on a question in its file carry an answer set?
pub const DEPENDS_ENTRY_HAS_ANSWER_SET: Rule = Rule {
    name: "depends_entry_has_answer_set",
    decisions: &["946263aa-519a-47aa-b743-5cdcf52f5d35"],
};

/// Decision: Does answer remove a question whose answer set lacks the answer?
pub const ANSWER_SET_ON_OPEN_QUESTION_ONLY: Rule = Rule {
    name: "answer_set_on_open_question_only",
    decisions: &["462e22c6-8b23-4061-9a2a-bbf8691fce60"],
};

/// Decision: What bounds the questions at one tier?
pub const TIER_QUESTION_LIMIT: Rule = Rule {
    name: "tier_question_limit",
    decisions: &["0e38f001-0226-4927-9a78-2e9fc1ec18d4"],
};

/// Decision: What bounds the questions a depends line names?
pub const DEPENDS_LINE_QUESTION_LIMIT: Rule = Rule {
    name: "depends_line_question_limit",
    decisions: &["010901e0-c02d-49eb-98bc-d584756d7f51"],
};

/// Decision: Does the agent write the bodies before approve?
pub const QUESTION_HAS_BODY: Rule = Rule {
    name: "question_has_body",
    decisions: &["f5eeaacf-1c9f-478b-8a97-2df1aaf39458"],
};

/// Decision: Does every neighbour get a line under its heading?
pub const NEIGHBOUR_IS_ON_A_LINE: Rule = Rule {
    name: "neighbour_is_on_a_line",
    decisions: &["c6451ecc-17a9-447e-a9c9-fe9b35ec789c"],
};

/// Decision: What does a reference in a topic file carry?
pub const UNRELATED_ENTRY_NAMES_ONE_DECISION: Rule = Rule {
    name: "unrelated_entry_names_one_decision",
    decisions: &["7c0b9fe3-6038-4a23-93fe-88a31c7eaf40"],
};

/// Decision: Does one decision supersede several?
pub const SUPERSEDES_TARGET_IS_CURRENT: Rule = Rule {
    name: "supersedes_target_is_current",
    decisions: &["f4f667ad-8714-4e65-897b-c7002e423c28"],
};

/// Decision: Does a supersedes line name every current decision standing on its target?
pub const SUPERSEDES_NAMES_DEPENDENTS: Rule = Rule {
    name: "supersedes_names_dependents",
    decisions: &["796a2544-43e5-43db-b5d3-8a4e737ec9c6"],
};

/// Decision: What does a reference in a topic file carry?
pub const SUPERSEDES_ENTRY_NAMES_ONE_DECISION: Rule = Rule {
    name: "supersedes_entry_names_one_decision",
    decisions: &["7c0b9fe3-6038-4a23-93fe-88a31c7eaf40"],
};

/// Decision: What does a question carry of its blame file?
pub const BLAME_LINE_NAMES_APPROVED_FILE: Rule = Rule {
    name: "blame_line_names_approved_file",
    decisions: &["69708a07-469c-406b-9e25-63da1605b432"],
};

/// Decision: What does a question carry of its blame file?
pub const BLAME_LINE_NAMES_ROW_RANGES: Rule = Rule {
    name: "blame_line_names_row_ranges",
    decisions: &["69708a07-469c-406b-9e25-63da1605b432"],
};

/// Decision: Does a banned word pass when it is only quoted?
pub const BANNED_WORD_ABSENT_FROM_RECORDS: Rule = Rule {
    name: "banned_word_absent_from_records",
    decisions: &["735a32bd-db0c-428e-b65e-2acc750671b0"],
};

/// Decision: Does ask refuse a question with no term?
pub const QUESTION_PAIRS_TO_TERM: Rule = Rule {
    name: "question_pairs_to_term",
    decisions: &["711a7636-9f4d-402d-ae62-265208adf0ac"],
};

/// Decision: Does ask stop a question outside the plan?
pub const PLAN_APPROVED_BEFORE_ASK: Rule = Rule {
    name: "plan_approved_before_ask",
    decisions: &["da59b448-1afb-4469-b40e-69eda8f253c2"],
};

/// Decision: Does ask run on a file with unapproved prose?
pub const PROSE_APPROVED_BEFORE_ASK: Rule = Rule {
    name: "prose_approved_before_ask",
    decisions: &["1cdec741-98e1-4715-a7a5-94f7a73cd425"],
};

/// Decision: Does a question keep its identifier when its heading is reworded?
pub const ID_NAMES_ONE_QUESTION: Rule = Rule {
    name: "id_names_one_question",
    decisions: &["7a1e7b81-0967-4ed2-8a8a-c7153056836f"],
};

/// Decision: Does approve withdraw an asked question struck from the plan?
/// Decision: Does a command refuse what check lets pass?
pub const ASKED_QUESTION_LEAVES_BY_APPROVAL: Rule = Rule {
    name: "asked_question_leaves_by_approval",
    decisions: &[
        "21ae8ae0-6884-4d3c-8464-024a8a5aae2d",
        "80b89c24-7d65-43f4-bb03-55e9d85c327d",
    ],
};

/// Decision: What may leave a topic file by a plan change?
/// Decision: Does a command refuse what check lets pass?
/// Decision: Does approve withdraw an asked question struck from the plan?
pub const FILE_LEAVES_WITH_NO_LIVE_QUESTION: Rule = Rule {
    name: "file_leaves_with_no_live_question",
    decisions: &[
        "79da6500-2246-4a80-baf1-0eaf7cad49ee",
        "80b89c24-7d65-43f4-bb03-55e9d85c327d",
        "21ae8ae0-6884-4d3c-8464-024a8a5aae2d",
    ],
};

/// Decision: Does approve withdraw an asked question struck from the plan?
pub const WITHDRAWN_ID_STAYS_OUT: Rule = Rule {
    name: "withdrawn_id_stays_out",
    decisions: &["21ae8ae0-6884-4d3c-8464-024a8a5aae2d"],
};

/// Decision: Does approve withdraw an asked question struck from the plan?
pub const ANSWERED_QUESTION_STAYS_IN_FILES: Rule = Rule {
    name: "answered_question_stays_in_files",
    decisions: &["21ae8ae0-6884-4d3c-8464-024a8a5aae2d"],
};

/// Decision: What may leave a topic file by a plan change?
pub const FILE_REMOVAL_KEEPS_LIVE_QUESTIONS: Rule = Rule {
    name: "file_removal_keeps_live_questions",
    decisions: &["79da6500-2246-4a80-baf1-0eaf7cad49ee"],
};

/// Decision: Does answer take a question standing on a question that has no decision?
pub const ANSWER_NEEDS_LOWER_ANSWERS: Rule = Rule {
    name: "answer_needs_lower_answers",
    decisions: &["da80931c-68cf-4c51-acce-060307bbd973"],
};

/// Decision: Does a refused question hold the tiers above it?
pub const ANSWER_NEEDS_NO_REFUSED_BELOW: Rule = Rule {
    name: "answer_needs_no_refused_below",
    decisions: &["506b3c6d-3d0a-4a02-98de-3b8a573d2fe9"],
};

/// Decision: Does check fail a question answered with two different options?
pub const ONE_STANDING_ANSWER: Rule = Rule {
    name: "one_standing_answer",
    decisions: &["e3200081-9995-4bbe-983f-2f430dc341e7"],
};

/// Decision: Does check fail an answered question whose text changed?
pub const ANSWERED_TEXT_FROZEN: Rule = Rule {
    name: "answered_text_frozen",
    decisions: &["cd83e277-ebf0-4458-a77a-c526ab4cfeca"],
};

/// Decision: Does a decision change after it is answered?
pub const ANSWER_LINE_KEPT: Rule = Rule {
    name: "answer_line_kept",
    decisions: &["dad5273c-afe2-4ab1-a451-e2a5d4d9a26d"],
};

/// Decision: Does a refused question stay in its topic file?
pub const REFUSAL_LINE_KEPT: Rule = Rule {
    name: "refusal_line_kept",
    decisions: &["6b3d8e59-e145-49e1-860d-a79d753d745c"],
};

/// Decision: What names a topic file?
pub const FILE_NAME_IS_LOWERCASE_HYPHENATED: Rule = Rule {
    name: "file_name_is_lowercase_hyphenated",
    decisions: &["625dea67-13a7-4e8e-913f-e997081a120c"],
};

/// Decision: What names a topic file?
pub const FILE_NAME_MATCHES_TOPIC: Rule = Rule {
    name: "file_name_matches_topic",
    decisions: &["625dea67-13a7-4e8e-913f-e997081a120c"],
};

/// Decision: What names a topic file?
pub const TOPIC_HAS_A_FILE_NAME: Rule = Rule {
    name: "topic_has_a_file_name",
    decisions: &["625dea67-13a7-4e8e-913f-e997081a120c"],
};

/// Decision: Does one question heading stand in two topic files?
pub const HEADING_IN_ONE_FILE: Rule = Rule {
    name: "heading_in_one_file",
    decisions: &["23f21671-e34c-44aa-8804-6163d975807f"],
};

/// Decision: What does approve take for a terms file holding several entries that wait?
pub const APPROVE_NAMES_ONE_TERM_ENTRY: Rule = Rule {
    name: "approve_names_one_term_entry",
    decisions: &["da0dc8b1-58cd-4a16-8e64-234e4bfd5320"],
};

/// Decision: What does a term hold?
pub const TERM_ENTRY_FIELDS: Rule = Rule {
    name: "term_entry_fields",
    decisions: &["7c306235-6243-4d79-b9aa-08ed19662779"],
};

/// Decision: What does a term hold?
pub const TERM_IDENTIFIER_IS_UNIQUE: Rule = Rule {
    name: "term_identifier_is_unique",
    decisions: &["7c306235-6243-4d79-b9aa-08ed19662779"],
};

/// Decision: Does a term word keep its plain sense in the records?
pub const WORD_NAMES_ONE_TERM: Rule = Rule {
    name: "word_names_one_term",
    decisions: &["521e6211-344a-4b56-a1aa-07be956b3b29"],
};

/// Decision: What does a definition say?
pub const DEFINITION_NOT_CIRCULAR: Rule = Rule {
    name: "definition_not_circular",
    decisions: &["fcc1c1ec-d8e8-428e-9731-9817cdeb8e76"],
};

/// Decision: What does a definition say?
pub const DEFINITION_SENTENCE_LIMIT: Rule = Rule {
    name: "definition_sentence_limit",
    decisions: &["fcc1c1ec-d8e8-428e-9731-9817cdeb8e76"],
};

/// Decision: Does approve stop a terms edit that moves a word an answered question pairs?
pub const TERM_STAYS_WHILE_USED: Rule = Rule {
    name: "term_stays_while_used",
    decisions: &["7cf81ded-be48-4777-b83e-f8a2998c1ab5"],
};

/// Decision: What holds a retire of a term?
pub const TERM_NOT_RETIRED_WHILE_NAMED: Rule = Rule {
    name: "term_not_retired_while_named",
    decisions: &["fcf9d2c6-009e-4d6a-a3e4-14274bd1ee94"],
};

/// Decision: What does a definition say?
pub const DEFINITION_USES_LIVE_TERMS: Rule = Rule {
    name: "definition_uses_live_terms",
    decisions: &["fcc1c1ec-d8e8-428e-9731-9817cdeb8e76"],
};

/// Decision: Does approve stop a terms edit that moves a word an answered question pairs?
pub const WORD_STAYS_ON_TERM: Rule = Rule {
    name: "word_stays_on_term",
    decisions: &["7cf81ded-be48-4777-b83e-f8a2998c1ab5"],
};

/// Decision: Does the tool stop while the terms file waits for approval?
pub const TERMS_FILE_APPROVED: Rule = Rule {
    name: "terms_file_approved",
    decisions: &["aa475aa3-05b2-429a-b33a-46b1dcb52049"],
};

/// Decision: Does check hold the terms brief to the terms file?
pub const TERMS_BRIEF_MATCHES_TERMS: Rule = Rule {
    name: "terms_brief_matches_terms",
    decisions: &["ec3f875c-a0f8-4085-8da1-fec9dcb14377"],
};

/// Decision: Does check at a commit read the staged files?
pub const STAGED_FILES_ARE_JUDGED_FILES: Rule = Rule {
    name: "staged_files_are_judged_files",
    decisions: &["34e81f57-9b3c-4dc4-a2ba-ca227523177d"],
};

/// Decision: What does a blame file hold of its origin?
pub const BLAME_FILE_SHAPE: Rule = Rule {
    name: "blame_file_shape",
    decisions: &["d2851ea1-fcb6-476a-b321-c813a594d5e5"],
};

/// Decision: What does a blame file hold of its origin?
pub const BLAME_ROW_SAYS_DECISION_OR_DETAIL: Rule = Rule {
    name: "blame_row_says_decision_or_detail",
    decisions: &["d2851ea1-fcb6-476a-b321-c813a594d5e5"],
};

/// Decision: Does a blame file cover its whole origin?
pub const BLAME_COVERS_ORIGIN: Rule = Rule {
    name: "blame_covers_origin",
    decisions: &["5c399a14-1e5c-48eb-b502-b57524c01c24"],
};

/// Decision: What does a blame file hold of its origin?
pub const BLAME_ORIGIN_IS_A_FILE: Rule = Rule {
    name: "blame_origin_is_a_file",
    decisions: &["d2851ea1-fcb6-476a-b321-c813a594d5e5"],
};

/// Decision: What does check do when an origin differs from its blame file's blob identifier?
pub const BLAME_BLOB_MATCHES_ORIGIN: Rule = Rule {
    name: "blame_blob_matches_origin",
    decisions: &["6f6e7f4a-d733-4f3a-99f9-6a47fec5db70"],
};

/// Decision: What does check do when an origin is absent?
pub const BLAME_ORIGIN_STAYS_UNTIL_APPROVED: Rule = Rule {
    name: "blame_origin_stays_until_approved",
    decisions: &["0c3d498a-c540-47f3-8384-050fc044c109"],
};

/// Decision: What does a blame file hold of its origin?
/// Decision: Does a question keep its identifier when its heading is reworded?
pub const BLAME_ID_ASSIGNED_BY_TOOL: Rule = Rule {
    name: "blame_id_assigned_by_tool",
    decisions: &[
        "d2851ea1-fcb6-476a-b321-c813a594d5e5",
        "7a1e7b81-0967-4ed2-8a8a-c7153056836f",
    ],
};

/// Decision: What does a blame file hold of its origin?
/// Decision: Does a question keep its identifier when its heading is reworded?
pub const BLAME_ID_NAMES_ONE_FILE: Rule = Rule {
    name: "blame_id_names_one_file",
    decisions: &[
        "d2851ea1-fcb6-476a-b321-c813a594d5e5",
        "7a1e7b81-0967-4ed2-8a8a-c7153056836f",
    ],
};

/// Decision: Does ask pair the whole question to terms?
pub const ASKED_QUESTION_HAS_TERMS_LINE: Rule = Rule {
    name: "asked_question_has_terms_line",
    decisions: &["f571a454-3e6c-46c1-b754-0822a5d1d667"],
};

/// Decision: Does every sentence of the brief come from a decision?
pub const BRIEF_SENTENCE_HAS_SOURCE: Rule = Rule {
    name: "brief_sentence_has_source",
    decisions: &["de5ed5b1-3a83-4403-8953-941cb7e94390"],
};

/// Decision: Does every sentence of the brief come from a decision?
pub const BRIEF_SOURCE_NAMES_CURRENT: Rule = Rule {
    name: "brief_source_names_current",
    decisions: &["de5ed5b1-3a83-4403-8953-941cb7e94390"],
};

/// Decision: Does check read the words of an answered question again?
pub const ASKED_TERMS_LINE_TERM_EXISTS: Rule = Rule {
    name: "asked_terms_line_term_exists",
    decisions: &["6108b8b2-911e-443b-abe0-4c6f8b407384"],
};

/// Decision: Does check read the words of an answered question again?
pub const ASKED_TERMS_LINE_TERM_IS_LIVE: Rule = Rule {
    name: "asked_terms_line_term_is_live",
    decisions: &["6108b8b2-911e-443b-abe0-4c6f8b407384"],
};

/// Decision: What does a definition say?
pub const DEFINITION_TERM_EXISTS: Rule = Rule {
    name: "definition_term_exists",
    decisions: &["fcc1c1ec-d8e8-428e-9731-9817cdeb8e76"],
};

/// Decision: Does check hold the files install writes?
pub const AGENTS_BRIEF_MATCHES_INSTALL: Rule = Rule {
    name: "agents_brief_matches_install",
    decisions: &["6ef9eca6-fd8b-4211-aae7-00b37a2a9245"],
};

/// Decision: Does check hold a satellite's brief to the primary's?
pub const SATELLITE_BRIEF_MATCHES_PRIMARY: Rule = Rule {
    name: "satellite_brief_matches_primary",
    decisions: &["03f47941-43c5-479f-bf64-2d030361092d"],
};

/// Decision: What does check in the primary report for a satellite missing at its path?
pub const SATELLITE_FOUND_AT_PATH: Rule = Rule {
    name: "satellite_found_at_path",
    decisions: &["fc118154-4172-48db-bddb-75a691894edf"],
};

/// Decision: What does install write for one agent?
pub const AGENT_FILE_IMPORTS_AGENTS_MD: Rule = Rule {
    name: "agent_file_imports_agents_md",
    decisions: &["ae63df95-0643-4108-af65-1e7a0009d631"],
};

/// Decision: What does install write for one agent?
pub const AGENT_SETTINGS_DENY_OWNER_COMMANDS: Rule = Rule {
    name: "agent_settings_deny_owner_commands",
    decisions: &["ae63df95-0643-4108-af65-1e7a0009d631"],
};

/// Decision: Does check hold the files install writes?
pub const AGENT_SETTINGS_FILE_PARSES: Rule = Rule {
    name: "agent_settings_file_parses",
    decisions: &["6ef9eca6-fd8b-4211-aae7-00b37a2a9245"],
};

/// Decision: Does check hold the files install writes?
pub const CODEX_RULES_MATCH_INSTALL: Rule = Rule {
    name: "codex_rules_match_install",
    decisions: &["6ef9eca6-fd8b-4211-aae7-00b37a2a9245"],
};

/// Decision: Does check hold the merge rule install writes for the ledger?
pub const GITATTRIBUTES_MATCH_INSTALL: Rule = Rule {
    name: "gitattributes_match_install",
    decisions: &["76c6adc6-c018-4df4-b49a-ddcd0a03a48c"],
};

/// Decision: What does a ledger row hold?
pub const LEDGER_ROW_FIELDS: Rule = Rule {
    name: "ledger_row_fields",
    decisions: &["f3e64c4e-21ab-442f-8c40-6060591e2b77"],
};

/// Decision: Does check fail a ledger row that was edited?
pub const LEDGER_ROWS_KEPT: Rule = Rule {
    name: "ledger_rows_kept",
    decisions: &["bf6697e3-8832-4b90-b036-8fecedb55406"],
};

/// Decision: Does check at a merge look for the ledger rows of both branches?
pub const MERGE_KEEPS_LEDGER_ROWS: Rule = Rule {
    name: "merge_keeps_ledger_rows",
    decisions: &["1c27f2d6-ab62-4c5a-91b2-1488722f3c48"],
};

/// Decision: What does answer write for a removed question?
pub const LEDGER_DROPPED_ROW_NAMES_ITS_CAUSE: Rule = Rule {
    name: "ledger_dropped_row_names_its_cause",
    decisions: &["f5c242b6-e2f6-45a3-8c61-c743f3ad794f"],
};

/// Decision: What does answer write for a removed question?
pub const LEDGER_DROPPED_HASH_IS_APPROVED: Rule = Rule {
    name: "ledger_dropped_hash_is_approved",
    decisions: &["f5c242b6-e2f6-45a3-8c61-c743f3ad794f"],
};

/// Decision: Does a decision change after it is answered?
pub const LEDGER_DROPPED_NOT_AFTER_CLOSE: Rule = Rule {
    name: "ledger_dropped_not_after_close",
    decisions: &["dad5273c-afe2-4ab1-a451-e2a5d4d9a26d"],
};

/// Decision: What does answer write for a removed question?
pub const LEDGER_DROPPED_HASH_IS_REMOVED_TEXT: Rule = Rule {
    name: "ledger_dropped_hash_is_removed_text",
    decisions: &["f5c242b6-e2f6-45a3-8c61-c743f3ad794f"],
};

/// Decision: Does answer remove a question whose answer set lacks the answer?
pub const LEDGER_DROPPED_LETTER_NOT_IN_SET: Rule = Rule {
    name: "ledger_dropped_letter_not_in_set",
    decisions: &["462e22c6-8b23-4061-9a2a-bbf8691fce60"],
};

/// Decision: Does check look at the ledger before a push?
pub const PUSH_KEEPS_LEDGER_ROWS: Rule = Rule {
    name: "push_keeps_ledger_rows",
    decisions: &["19e1c023-a678-4842-a414-bb0e45c41601"],
};

/// Decision: Does check in CI compare the ledger with the commit before the push?
pub const SINCE_KEEPS_LEDGER_ROWS: Rule = Rule {
    name: "since_keeps_ledger_rows",
    decisions: &["a26876ba-5789-4d05-965e-853d00d9edd2"],
};

/// Decision: Does check fail two ledger rows on one subject with the same time?
pub const LEDGER_LATEST_ROW_IS_ONE: Rule = Rule {
    name: "ledger_latest_row_is_one",
    decisions: &["ce19c883-c61d-477b-a6d4-fee5134c6e9a"],
};

/// Decision: What does the tool keep of a reader's word?
pub const READER_ROW_FIELDS: Rule = Rule {
    name: "reader_row_fields",
    decisions: &["55a6a2d8-03af-4db9-a4c7-e2790e38357e"],
};

/// Decision: What does check hold of a reader's file?
pub const READER_ROWS_UNEDITED: Rule = Rule {
    name: "reader_rows_unedited",
    decisions: &["30ca5d13-e68b-465f-aa84-ea4000c9330a"],
};

/// Decision: What does check hold of a reader's file?
pub const MERGE_LEAVES_READER_ROWS_UNEDITED: Rule = Rule {
    name: "merge_leaves_reader_rows_unedited",
    decisions: &["30ca5d13-e68b-465f-aa84-ea4000c9330a"],
};

/// Decision: What does check hold of a reader's file?
pub const PUSH_LEAVES_READER_ROWS_UNEDITED: Rule = Rule {
    name: "push_leaves_reader_rows_unedited",
    decisions: &["30ca5d13-e68b-465f-aa84-ea4000c9330a"],
};

/// Decision: What does check hold of a reader's file?
pub const SINCE_LEAVES_READER_ROWS_UNEDITED: Rule = Rule {
    name: "since_leaves_reader_rows_unedited",
    decisions: &["30ca5d13-e68b-465f-aa84-ea4000c9330a"],
};

/// Decision: What does the ban scan read?
pub const BANNED_WORD_ABSENT_FROM_FILES: Rule = Rule {
    name: "banned_word_absent_from_files",
    decisions: &["d8a9dfff-de89-442c-a08c-8276efab3f36"],
};

/// Decision: Does check hold the ban list equal to the ledger?
pub const BAN_LIST_MATCHES_LEDGER: Rule = Rule {
    name: "ban_list_matches_ledger",
    decisions: &["06b7e003-4d9f-427a-a798-c1af97865411"],
};

/// Decision: Does answer remove a question whose answer set lacks the answer?
pub const ANSWER_SET_FROZEN_AT_ASK: Rule = Rule {
    name: "answer_set_frozen_at_ask",
    decisions: &["462e22c6-8b23-4061-9a2a-bbf8691fce60"],
};

/// Decision: What does check do with an untested current decision?
pub const DECISION_HAS_TEST_TIE: Rule = Rule {
    name: "decision_has_test_tie",
    decisions: &["3460d220-e523-448b-aeff-baa8051b7d06"],
};

/// Decision: What does check do with a test that names a decision that is not current?
pub const TEST_TIE_NAMES_CURRENT_DECISION: Rule = Rule {
    name: "test_tie_names_current_decision",
    decisions: &["2381d759-f85f-484e-9031-fadbe6ea045a"],
};

/// Decision: Does check read the words of an answered question again?
pub const ASKED_TERMS_LINE_PARSES: Rule = Rule {
    name: "asked_terms_line_parses",
    decisions: &["6108b8b2-911e-443b-abe0-4c6f8b407384"],
};

/// Decision: Does check read the words of an answered question again?
pub const ANSWERED_TERMS_LINE_TERM_EXISTS: Rule = Rule {
    name: "answered_terms_line_term_exists",
    decisions: &["6108b8b2-911e-443b-abe0-4c6f8b407384"],
};

/// Decision: Does check read the words of an answered question again?
pub const ANSWERED_TERMS_LINE_TERM_IS_LIVE: Rule = Rule {
    name: "answered_terms_line_term_is_live",
    decisions: &["6108b8b2-911e-443b-abe0-4c6f8b407384"],
};

/// Decision: Does a decision change after it is answered?
pub const ANSWER_NEVER_REMOVES_CLOSED_QUESTION: Rule = Rule {
    name: "answer_never_removes_closed_question",
    decisions: &["dad5273c-afe2-4ab1-a451-e2a5d4d9a26d"],
};

/// Decision: Does ban refuse a word that appears in the repository?
pub const BAN_NOT_ON_USED_WORD: Rule = Rule {
    name: "ban_not_on_used_word",
    decisions: &["4f04bae4-98e3-48eb-9403-3257268e2d87"],
};

/// Decision: Does the tool print each pair the reader marks? A pair the
/// reader marks the same. The two pair rules stand in `ALL` in the order
/// of the reader's words, so `adr read` prints same, then inconsistent.
pub const READ_PAIR_SAME: Rule = Rule {
    name: "read_pair_same",
    decisions: &["27bd0855-a6c8-4cda-a82b-bf95022a14b0"],
};

/// Decision: Does the tool print each pair the reader marks? A pair the
/// reader marks inconsistent.
pub const READ_PAIR_INCONSISTENT: Rule = Rule {
    name: "read_pair_inconsistent",
    decisions: &["27bd0855-a6c8-4cda-a82b-bf95022a14b0"],
};

/// Decision: Does an owner command commit?
pub const ANSWER_LINE_HAS_ROW: Rule = Rule {
    name: "answer_line_has_row",
    decisions: &["dfd32ef6-c2ac-40ef-b8b3-9c58732516b1"],
};

/// Decision: Does an owner command commit?
/// Decision: Does the owner answer a question with a refusal?
pub const REFUSAL_LINE_HAS_ROW: Rule = Rule {
    name: "refusal_line_has_row",
    decisions: &[
        "dfd32ef6-c2ac-40ef-b8b3-9c58732516b1",
        "b782664f-7465-4e1e-99e0-13f6dd2de774",
    ],
};

/// Decision: Does check fail an answered question whose text changed?
/// Decision: Does the owner answer a question with a refusal?
pub const REFUSED_TEXT_FROZEN: Rule = Rule {
    name: "refused_text_frozen",
    decisions: &[
        "cd83e277-ebf0-4458-a77a-c526ab4cfeca",
        "b782664f-7465-4e1e-99e0-13f6dd2de774",
    ],
};

/// Decision: Does a blame file cover its whole origin?
pub const BLAME_ROW_RANGE_WITHIN_ORIGIN: Rule = Rule {
    name: "blame_row_range_within_origin",
    decisions: &["5c399a14-1e5c-48eb-b502-b57524c01c24"],
};

/// Decision: What does the agent do to plan a topic file?
/// Decision: Does a planning step run without the record of the step before?
pub const PLAN_STEP_HAS_ITS_RECORD: Rule = Rule {
    name: "plan_step_has_its_record",
    decisions: &[
        "46fc51ff-66b9-4841-a953-ae0d4c2fc424",
        "d3be71e4-ce7b-43b0-86a8-e86868521f99",
    ],
};

/// Decision: What does answer write for a removed question?
pub const DROP_NAMES_AN_IDENTIFIER: Rule = Rule {
    name: "drop_names_an_identifier",
    decisions: &["f5c242b6-e2f6-45a3-8c61-c743f3ad794f"],
};

/// Decision: What does answer write for a removed question?
pub const DROP_HAS_APPROVED_TEXT: Rule = Rule {
    name: "drop_has_approved_text",
    decisions: &["f5c242b6-e2f6-45a3-8c61-c743f3ad794f"],
};

/// Decision: Does answer remove a question whose answer set lacks the answer?
pub const DROPPED_QUESTION_LEAVES_FILES: Rule = Rule {
    name: "dropped_question_leaves_files",
    decisions: &["462e22c6-8b23-4061-9a2a-bbf8691fce60"],
};

/// Decision: What strikes the section of a superseded question from its topic file?
pub const SUPERSEDED_QUESTION_LEAVES_FILES: Rule = Rule {
    name: "superseded_question_leaves_files",
    decisions: &["b5ca161f-c88c-4f02-a9d9-1d406cf3a252"],
};

/// Decision: What are the tiers derived from?
pub const DEPENDS_LINES_HAVE_NO_CYCLE: Rule = Rule {
    name: "depends_lines_have_no_cycle",
    decisions: &["f1b4c4cc-97bd-446f-87b4-df5f1b6f4da2"],
};

/// Decision: What are the tiers derived from?
pub const QUESTION_NOT_ON_ITSELF: Rule = Rule {
    name: "question_not_on_itself",
    decisions: &["f1b4c4cc-97bd-446f-87b4-df5f1b6f4da2"],
};

/// Decision: Does check hold the files install writes?
/// Decision: Does install write outside adrdom?
pub const HOOK_MATCHES_INSTALL: Rule = Rule {
    name: "hook_matches_install",
    decisions: &[
        "6ef9eca6-fd8b-4211-aae7-00b37a2a9245",
        "4f925b13-ad16-48b8-acd1-3e30d6e308b6",
    ],
};

/// Decision: Does check hold git's hooksPath?
/// Decision: Does install write outside adrdom?
pub const HOOKS_PATH_IS_GITHOOKS: Rule = Rule {
    name: "hooks_path_is_githooks",
    decisions: &[
        "db76c9a0-b4fc-4aa2-8afb-47cd78ef7138",
        "4f925b13-ad16-48b8-acd1-3e30d6e308b6",
    ],
};

/// Decision: Does check hold the files install writes?
/// Decision: Does install write outside adrdom?
pub const HOOK_IS_EXECUTABLE: Rule = Rule {
    name: "hook_is_executable",
    decisions: &[
        "6ef9eca6-fd8b-4211-aae7-00b37a2a9245",
        "4f925b13-ad16-48b8-acd1-3e30d6e308b6",
    ],
};

/// Decision: Does check read the words of an answered question again?
/// Decision: Does ask pair the whole question to terms?
pub const SECTION_TERMS_LINE_PARSES: Rule = Rule {
    name: "section_terms_line_parses",
    decisions: &[
        "6108b8b2-911e-443b-abe0-4c6f8b407384",
        "f571a454-3e6c-46c1-b754-0822a5d1d667",
    ],
};

/// Decision: What does a definition say?
/// Decision: Does check read the words of an answered question again?
pub const DEFINITION_TERMS_LINE_PARSES: Rule = Rule {
    name: "definition_terms_line_parses",
    decisions: &[
        "fcc1c1ec-d8e8-428e-9731-9817cdeb8e76",
        "6108b8b2-911e-443b-abe0-4c6f8b407384",
    ],
};

/// Decision: Does check read the words of an answered question again?
/// Decision: Does ask pair the whole question to terms?
pub const RETIRE_READS_ASKED_TERMS_LINES: Rule = Rule {
    name: "retire_reads_asked_terms_lines",
    decisions: &[
        "6108b8b2-911e-443b-abe0-4c6f8b407384",
        "f571a454-3e6c-46c1-b754-0822a5d1d667",
    ],
};

/// Decision: What holds the list of loaded phrases?
pub const TELL_NAMES_SPEAKER: Rule = Rule {
    name: "tell_names_speaker",
    decisions: &["149c9a73-16eb-46aa-9806-2950423ee1a2"],
};

/// Decision: What holds the list of loaded phrases?
pub const TELL_STATES_CODE_NOW: Rule = Rule {
    name: "tell_states_code_now",
    decisions: &["149c9a73-16eb-46aa-9806-2950423ee1a2"],
};

/// Decision: What holds the list of loaded phrases?
pub const TELL_CARRIES_DATE: Rule = Rule {
    name: "tell_carries_date",
    decisions: &["149c9a73-16eb-46aa-9806-2950423ee1a2"],
};

/// Decision: What holds the list of loaded phrases?
pub const TELL_ASSUMES_ANSWER: Rule = Rule {
    name: "tell_assumes_answer",
    decisions: &["149c9a73-16eb-46aa-9806-2950423ee1a2"],
};

/// Decision: What holds the list of loaded phrases?
pub const TELL_JOINS_TWO_QUESTIONS: Rule = Rule {
    name: "tell_joins_two_questions",
    decisions: &["149c9a73-16eb-46aa-9806-2950423ee1a2"],
};

/// Decision: What stands under a question's heading?
/// Decision: What is the shape of a topic file?
pub const ANSWER_LINE_IS_LAST: Rule = Rule {
    name: "answer_line_is_last",
    decisions: &[
        "178a232e-b51c-4122-a1a9-9e9ed6970748",
        "9913568f-6b85-4d26-946b-c6e108f9aa51",
    ],
};

/// Decision: Does the owner answer a question with a refusal?
pub const ANSWER_NAMES_AN_OPTION: Rule = Rule {
    name: "answer_names_an_option",
    decisions: &["b782664f-7465-4e1e-99e0-13f6dd2de774"],
};

/// Decision: Does a depends entry on a question in its file carry an answer set?
pub const ANSWER_SET_NAMES_TARGET_OPTIONS: Rule = Rule {
    name: "answer_set_names_target_options",
    decisions: &["946263aa-519a-47aa-b743-5cdcf52f5d35"],
};

/// Decision: Does a depends entry on a question in its file carry an answer set?
pub const ANSWER_SET_NOT_ON_TERM: Rule = Rule {
    name: "answer_set_not_on_term",
    decisions: &["946263aa-519a-47aa-b743-5cdcf52f5d35"],
};

/// Decision: What does a reference in a topic file carry?
pub const DEPENDS_ENTRY_ONCE: Rule = Rule {
    name: "depends_entry_once",
    decisions: &["7c0b9fe3-6038-4a23-93fe-88a31c7eaf40"],
};

/// Decision: What does a reference in a topic file carry?
pub const SUPERSEDES_ENTRY_ONCE: Rule = Rule {
    name: "supersedes_entry_once",
    decisions: &["7c0b9fe3-6038-4a23-93fe-88a31c7eaf40"],
};

/// Decision: What does a depends entry name?
/// Decision: Does one decision supersede several?
pub const QUESTION_NOT_ON_WHAT_IT_SUPERSEDES: Rule = Rule {
    name: "question_not_on_what_it_supersedes",
    decisions: &[
        "9afe335c-2a7a-4625-b115-f74d3f091cac",
        "f4f667ad-8714-4e65-897b-c7002e423c28",
    ],
};

/// Decision: What does a depends entry name?
/// Decision: Does one decision supersede several?
pub const NO_QUESTION_ON_WHAT_PLAN_SUPERSEDES: Rule = Rule {
    name: "no_question_on_what_plan_supersedes",
    decisions: &[
        "9afe335c-2a7a-4625-b115-f74d3f091cac",
        "f4f667ad-8714-4e65-897b-c7002e423c28",
    ],
};

/// Decision: What does a reference in a topic file carry?
pub const HEADING_ONCE_IN_FILE: Rule = Rule {
    name: "heading_once_in_file",
    decisions: &["7c0b9fe3-6038-4a23-93fe-88a31c7eaf40"],
};

/// Decision: What does a reference in a topic file carry?
pub const UNRELATED_ENTRY_NAMES_SOMETHING: Rule = Rule {
    name: "unrelated_entry_names_something",
    decisions: &["7c0b9fe3-6038-4a23-93fe-88a31c7eaf40"],
};

/// Decision: What does a term hold?
pub const TERMS_FILE_SHAPE: Rule = Rule {
    name: "terms_file_shape",
    decisions: &["7c306235-6243-4d79-b9aa-08ed19662779"],
};

/// Decision: Does ask refuse a question with no term?
pub const TERMS_FILE_EXISTS: Rule = Rule {
    name: "terms_file_exists",
    decisions: &["711a7636-9f4d-402d-ae62-265208adf0ac"],
};

/// Decision: Does git track a file the tool can get again without the owner?
pub const GITIGNORE_MATCHES_INSTALL: Rule = Rule {
    name: "gitignore_matches_install",
    decisions: &["0d977e39-a657-4596-8eca-8ceb335f3081"],
};

/// Decision: Does an answer carry the version of the question the owner read?
pub const ASKED_TEXT_FROZEN: Rule = Rule {
    name: "asked_text_frozen",
    decisions: &["11be073c-ce91-44d8-833a-ef0b5e0b1672"],
};

/// Decision: Does an answer carry the version of the question the owner read?
pub const ANSWER_ROW_HAS_ASKED_ROW: Rule = Rule {
    name: "answer_row_has_asked_row",
    decisions: &["11be073c-ce91-44d8-833a-ef0b5e0b1672"],
};

/// Decision: What does answer write for a removed question?
pub const DROPPED_ROW_HAS_CAUSE: Rule = Rule {
    name: "dropped_row_has_cause",
    decisions: &["f5c242b6-e2f6-45a3-8c61-c743f3ad794f"],
};

/// Decision: What does answer write for a removed question?
pub const DROPPED_ROW_NAMES_GROUND: Rule = Rule {
    name: "dropped_row_names_ground",
    decisions: &["f5c242b6-e2f6-45a3-8c61-c743f3ad794f"],
};

/// Decision: Does a command refuse what check lets pass?
/// Decision: Does approve withdraw an asked question struck from the plan?
pub const LEDGER_IDENTIFIER_IN_A_FILE: Rule = Rule {
    name: "ledger_identifier_in_a_file",
    decisions: &[
        "80b89c24-7d65-43f4-bb03-55e9d85c327d",
        "21ae8ae0-6884-4d3c-8464-024a8a5aae2d",
    ],
};

/// Decision: Does check fail a planner's commit that reaches outside adrdom?
pub const PLANNER_COMMIT_STAYS_IN_ADRDOM: Rule = Rule {
    name: "planner_commit_stays_in_adrdom",
    decisions: &["db07a54d-4d80-4035-a9ef-eabcc0c872b3"],
};

/// Every rule, for the test that holds each to its decisions.
pub const ALL: &[&Rule] = &[
    &TOPIC_FILE_SHAPE,
    &TIER_HEADING_ABSENT,
    &QUESTION_HEADING_ENDS_IN_MARK,
    &QUESTION_SECTION_SHAPE,
    &REFERENCE_LINE_ONCE,
    &OPTION_IS_ONE_SENTENCE,
    &ID_ASSIGNED_BY_TOOL,
    &DOES_OPTION_IS_ONE_WORD,
    &OPTION_WORD_LIMIT,
    &WHY_WORD_LIMIT,
    &COSTS_WORD_LIMIT,
    &QUESTION_HAS_DEPENDS_LINE,
    &DEPENDS_ENTRY_RESOLVES,
    &DEPENDS_ENTRY_NAMES_TERM_OR_FILE_QUESTION,
    &DEPENDS_TERM_IS_LIVE,
    &DEPENDS_TERM_EXISTS,
    &ANSWERED_DEPENDS_LINE_FROZEN,
    &DEPENDS_ENTRY_HAS_ANSWER_SET,
    &ANSWER_SET_ON_OPEN_QUESTION_ONLY,
    &TIER_QUESTION_LIMIT,
    &DEPENDS_LINE_QUESTION_LIMIT,
    &QUESTION_HAS_BODY,
    &NEIGHBOUR_IS_ON_A_LINE,
    &UNRELATED_ENTRY_NAMES_ONE_DECISION,
    &SUPERSEDES_TARGET_IS_CURRENT,
    &SUPERSEDES_NAMES_DEPENDENTS,
    &SUPERSEDES_ENTRY_NAMES_ONE_DECISION,
    &BLAME_LINE_NAMES_APPROVED_FILE,
    &BLAME_LINE_NAMES_ROW_RANGES,
    &BANNED_WORD_ABSENT_FROM_RECORDS,
    &QUESTION_PAIRS_TO_TERM,
    &PLAN_APPROVED_BEFORE_ASK,
    &PROSE_APPROVED_BEFORE_ASK,
    &ID_NAMES_ONE_QUESTION,
    &ASKED_QUESTION_LEAVES_BY_APPROVAL,
    &FILE_LEAVES_WITH_NO_LIVE_QUESTION,
    &WITHDRAWN_ID_STAYS_OUT,
    &ANSWERED_QUESTION_STAYS_IN_FILES,
    &FILE_REMOVAL_KEEPS_LIVE_QUESTIONS,
    &ANSWER_NEEDS_LOWER_ANSWERS,
    &ANSWER_NEEDS_NO_REFUSED_BELOW,
    &ONE_STANDING_ANSWER,
    &ANSWERED_TEXT_FROZEN,
    &ANSWER_LINE_KEPT,
    &REFUSAL_LINE_KEPT,
    &FILE_NAME_IS_LOWERCASE_HYPHENATED,
    &FILE_NAME_MATCHES_TOPIC,
    &TOPIC_HAS_A_FILE_NAME,
    &HEADING_IN_ONE_FILE,
    &APPROVE_NAMES_ONE_TERM_ENTRY,
    &TERM_ENTRY_FIELDS,
    &TERM_IDENTIFIER_IS_UNIQUE,
    &WORD_NAMES_ONE_TERM,
    &DEFINITION_NOT_CIRCULAR,
    &DEFINITION_SENTENCE_LIMIT,
    &TERM_STAYS_WHILE_USED,
    &TERM_NOT_RETIRED_WHILE_NAMED,
    &DEFINITION_USES_LIVE_TERMS,
    &WORD_STAYS_ON_TERM,
    &TERMS_FILE_APPROVED,
    &TERMS_BRIEF_MATCHES_TERMS,
    &STAGED_FILES_ARE_JUDGED_FILES,
    &BLAME_FILE_SHAPE,
    &BLAME_ROW_SAYS_DECISION_OR_DETAIL,
    &BLAME_COVERS_ORIGIN,
    &BLAME_ORIGIN_IS_A_FILE,
    &BLAME_BLOB_MATCHES_ORIGIN,
    &BLAME_ORIGIN_STAYS_UNTIL_APPROVED,
    &BLAME_ID_ASSIGNED_BY_TOOL,
    &BLAME_ID_NAMES_ONE_FILE,
    &ASKED_QUESTION_HAS_TERMS_LINE,
    &ASKED_TERMS_LINE_TERM_EXISTS,
    &ASKED_TERMS_LINE_TERM_IS_LIVE,
    &DEFINITION_TERM_EXISTS,
    &AGENTS_BRIEF_MATCHES_INSTALL,
    &SATELLITE_BRIEF_MATCHES_PRIMARY,
    &SATELLITE_FOUND_AT_PATH,
    &AGENT_FILE_IMPORTS_AGENTS_MD,
    &AGENT_SETTINGS_DENY_OWNER_COMMANDS,
    &AGENT_SETTINGS_FILE_PARSES,
    &CODEX_RULES_MATCH_INSTALL,
    &GITATTRIBUTES_MATCH_INSTALL,
    &LEDGER_ROW_FIELDS,
    &LEDGER_ROWS_KEPT,
    &MERGE_KEEPS_LEDGER_ROWS,
    &LEDGER_DROPPED_ROW_NAMES_ITS_CAUSE,
    &LEDGER_DROPPED_HASH_IS_APPROVED,
    &LEDGER_DROPPED_NOT_AFTER_CLOSE,
    &LEDGER_DROPPED_HASH_IS_REMOVED_TEXT,
    &LEDGER_DROPPED_LETTER_NOT_IN_SET,
    &PUSH_KEEPS_LEDGER_ROWS,
    &SINCE_KEEPS_LEDGER_ROWS,
    &LEDGER_LATEST_ROW_IS_ONE,
    &READER_ROW_FIELDS,
    &READER_ROWS_UNEDITED,
    &MERGE_LEAVES_READER_ROWS_UNEDITED,
    &PUSH_LEAVES_READER_ROWS_UNEDITED,
    &SINCE_LEAVES_READER_ROWS_UNEDITED,
    &BANNED_WORD_ABSENT_FROM_FILES,
    &BAN_LIST_MATCHES_LEDGER,
    &ANSWER_SET_FROZEN_AT_ASK,
    &DECISION_HAS_TEST_TIE,
    &TEST_TIE_NAMES_CURRENT_DECISION,
    &BRIEF_SENTENCE_HAS_SOURCE,
    &BRIEF_SOURCE_NAMES_CURRENT,
    &ASKED_TERMS_LINE_PARSES,
    &ANSWERED_TERMS_LINE_TERM_EXISTS,
    &ANSWERED_TERMS_LINE_TERM_IS_LIVE,
    &ANSWER_NEVER_REMOVES_CLOSED_QUESTION,
    &BAN_NOT_ON_USED_WORD,
    &READ_PAIR_SAME,
    &READ_PAIR_INCONSISTENT,
    &ANSWER_LINE_HAS_ROW,
    &REFUSAL_LINE_HAS_ROW,
    &REFUSED_TEXT_FROZEN,
    &BLAME_ROW_RANGE_WITHIN_ORIGIN,
    &PLAN_STEP_HAS_ITS_RECORD,
    &DROP_NAMES_AN_IDENTIFIER,
    &DROP_HAS_APPROVED_TEXT,
    &DROPPED_QUESTION_LEAVES_FILES,
    &SUPERSEDED_QUESTION_LEAVES_FILES,
    &DEPENDS_LINES_HAVE_NO_CYCLE,
    &QUESTION_NOT_ON_ITSELF,
    &HOOK_MATCHES_INSTALL,
    &HOOK_IS_EXECUTABLE,
    &HOOKS_PATH_IS_GITHOOKS,
    &SECTION_TERMS_LINE_PARSES,
    &DEFINITION_TERMS_LINE_PARSES,
    &RETIRE_READS_ASKED_TERMS_LINES,
    &TELL_NAMES_SPEAKER,
    &TELL_STATES_CODE_NOW,
    &TELL_CARRIES_DATE,
    &TELL_ASSUMES_ANSWER,
    &TELL_JOINS_TWO_QUESTIONS,
    &ANSWER_LINE_IS_LAST,
    &ANSWER_NAMES_AN_OPTION,
    &ANSWER_SET_NAMES_TARGET_OPTIONS,
    &ANSWER_SET_NOT_ON_TERM,
    &DEPENDS_ENTRY_ONCE,
    &SUPERSEDES_ENTRY_ONCE,
    &QUESTION_NOT_ON_WHAT_IT_SUPERSEDES,
    &NO_QUESTION_ON_WHAT_PLAN_SUPERSEDES,
    &HEADING_ONCE_IN_FILE,
    &UNRELATED_ENTRY_NAMES_SOMETHING,
    &TERMS_FILE_SHAPE,
    &TERMS_FILE_EXISTS,
    &GITIGNORE_MATCHES_INSTALL,
    &ASKED_TEXT_FROZEN,
    &ANSWER_ROW_HAS_ASKED_ROW,
    &DROPPED_ROW_HAS_CAUSE,
    &DROPPED_ROW_NAMES_GROUND,
    &LEDGER_IDENTIFIER_IN_A_FILE,
    &PLANNER_COMMIT_STAYS_IN_ADRDOM,
];
