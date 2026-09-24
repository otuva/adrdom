# Ledger

What one ledger row holds, what identifies a question, and what keeps the ledger append-only.

### What does a ledger row hold?
depends: e18b82d8-5271-4385-bbab-2b89e5da278f
id: f3e64c4e-21ab-442f-8c40-6060591e2b77
terms: approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: the ledger is the record of every owner act and every ask; a row that cannot say when, what, which question and which bytes proves nothing later.
Standard: append-only audit logs write one line per event with a timestamp, an event type, the subject and a digest of the content.

- A) One tab-separated row per event in one append-only file: the time, the event, the identifier, a hash of the text it covers, a detail, and the tool version.
  Why: one line per event diffs cleanly in git and reads with any text tool.
  Costs: rows from two branches can conflict at the end of the file.
- B) One JSON object per line with the same fields.
  Why: named fields and room to grow.
  Costs: noisier diffs and a parser for a flat record.
- C) One file per question.
  Why: no conflicts at the end of one shared file.
  Costs: an event that spans questions, such as an approve, has no single home.

**Answer:** A

### Does a question keep its identifier when its heading is reworded?
depends: 277a8338-c57a-43cc-8b28-6b42e9347f6a, e877e4a9-2caf-4fe6-9f82-5476a2abb00b
id: 7a1e7b81-0967-4ed2-8a8a-c7153056836f
terms: identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: a question known by a slug or by its heading loses its identity on every rename, and every reference to it breaks.
Standard: stable keys; a record's identity is a value assigned once, apart from its title and location.

- A) Yes: the identifier is assigned once, never reused, and independent of the file, the heading and the wording; the text is hashed separately, so an edit is still caught.
  Why: identity survives renames and moves, and the hash still catches an edit.
  Costs: the identifier is unreadable in conversation, so every mention needs the heading beside it.
- B) No: the owner picks a slug for each question, and a rename is an owner act.
  Why: readable names.
  Costs: a rename is a change of state, and a slug invites its meaning to drift.

**Answer:** A

### Does every ledger row carry the tool version?
depends: e18b82d8-5271-4385-bbab-2b89e5da278f
id: 99619e27-19b6-47ef-83b9-c79ac30ee2f0
terms: ledger=e18b82d8-5271-4385-bbab-2b89e5da278f

Problem: when the tool's rules change between releases, a row cannot say which rules were in force when it was written.
Standard: records written by versioned software carry the writer's version, taken from one source of truth.

- A) Yes: a VERSION file at the repository root is the one source of the version; the binary embeds it, adr --version prints it, and every ledger row carries it.
  Why: any row says which release wrote it, which a later migration needs.
  Costs: one more field on every row.
- B) No: the version lives only in the binary.
  Why: shorter rows.
  Costs: a row cannot say which release wrote it.

**Answer:** A

### Does check fail a ledger row that was edited?
depends: e18b82d8-5271-4385-bbab-2b89e5da278f, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de
id: bf6697e3-8832-4b90-b036-8fecedb55406
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, agents=f90dbd5b-8fbe-4899-8588-f8762795c2e2, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d

Problem: the ledger is a text file an agent can edit; an edited row rewrites history the owner made.
Standard: append-only logs are verified against their last committed state; anything but new lines at the end is tampering.

- A) Yes: every row present at HEAD must be present, byte for byte, in the working copy; only new rows pass.
  Why: an edit or a deletion of a past event fails the next commit.
  Costs: a hand-forged new row still passes; only the deny rules keep agents from writing it.
- B) No: git history shows any edit, and review catches it.
  Why: nothing new in the tool.
  Costs: history the owner made can be rewritten in one commit.

**Answer:** A
