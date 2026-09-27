# Blame

How the text of an origin enters the records so that every line of it is accounted for: a blame file under adrdom/blame holds, for one origin, its line ranges and what each range went to. The map is the proof; git keeps the text. Agents write and verify the map, the owner approves it.

### What does a blame file hold?
depends: 108053a8-c2c3-4561-874c-2f573a5409f2, ecb18413-cb3e-437e-9444-3cdd406ed67d
id: 34f0202a-0be4-4049-94de-d3886f4f8ffc
terms: blame=108053a8-c2c3-4561-874c-2f573a5409f2, blame file=108053a8-c2c3-4561-874c-2f573a5409f2, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, origin=ecb18413-cb3e-437e-9444-3cdd406ed67d, topic files=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: the map between an origin's text and the records lives only in the session that wrote it, and nobody can later tell what was left out.
Standard: git blame attributes each line range of a file to a commit by reference, never by copying the text.

- A) The origin's path and git blob identifier, then one row per line range of the origin naming what the range went to, with no text of the origin copied.
  Why: the origin's text stays in git under a fixed reference, and the file stays readable at any size.
  Costs: the file cannot be read without the origin or git beside it.
- B) One row per passage quoted from the origin, each naming what the passage went to.
  Why: the file reads on its own.
  Costs: the origin's text is written a second time, and a quote can drift from its origin.
- C) One line per origin naming the topic files its text went to.
  Why: the smallest map.
  Costs: nothing says which passage went to which record or was dropped.

**Answer:** A

### Does a blame file cover its whole origin?
depends: 108053a8-c2c3-4561-874c-2f573a5409f2, ecb18413-cb3e-437e-9444-3cdd406ed67d
id: 5c399a14-1e5c-48eb-b502-b57524c01c24
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, blame file=108053a8-c2c3-4561-874c-2f573a5409f2, origin=ecb18413-cb3e-437e-9444-3cdd406ed67d

Problem: a passage no row names is dropped without anyone deciding it.
Standard: a reconciliation is complete only when every source item is accounted for, and a gap is a failure, not a note.

- A) Yes: the ranges reach every line of the origin with no gap, a range may appear in more than one row, and approve stops a file with a gap.
  Why: every passage has at least one named destination, and one passage may feed two records.
  Costs: headings, blank lines and boilerplate need rows too.
- B) Yes, and every line sits in exactly one row.
  Why: one destination per passage, no double counting.
  Costs: a passage that feeds two records is split or forced to one.
- C) No: rows name only some passages, and the rest is unmapped.
  Why: the agent writes fewer rows.
  Costs: what nobody named is lost silently, the failure the file exists to stop.

**Answer:** A

### What does a blame row name?
depends: 108053a8-c2c3-4561-874c-2f573a5409f2, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, f590059d-b681-40ec-8a1c-c8916d71f6bb, 08193e75-dd89-4857-8e9f-b19e1a9e75e6
id: 38216d39-b064-4927-ab60-951a3c89e388
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, blame=108053a8-c2c3-4561-874c-2f573a5409f2, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, checked=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, identifiers=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, origin=ecb18413-cb3e-437e-9444-3cdd406ed67d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, term=f590059d-b681-40ec-8a1c-c8916d71f6bb, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: a destination the tool does not know cannot be checked, and an agent can invent one to hide a drop.
Standard: a controlled vocabulary is a closed set of values, each with a defined referent, and anything else is rejected.

- A) One of a closed set: a question by heading, a term by name, a topic file for its prose, or a drop marked dead, detail, or copy of a question or of another origin, and approve turns headings and names into identifiers and stops any other value.
  Why: every row resolves to something the tool can find, and a copy of a passage whose decision lives in an origin not yet mapped still has a target.
  Costs: a new kind of destination is a new question.
- B) A question, a term or a topic file, and nothing else, so a passage that goes nowhere has no row.
  Why: every row points at a record.
  Costs: a drop has no reason, and the coverage question loses its meaning.
- C) Free text.
  Why: the agent says what happened in its own words.
  Costs: approve can check nothing.

**Answer:** A

### Does the owner approve a blame file?
depends: 108053a8-c2c3-4561-874c-2f573a5409f2, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, 1d7e59ea-99e3-4d4b-9928-5dabca62723d
id: e879fe72-ec07-48cd-9902-2b158aaae726
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, agents=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, blame file=108053a8-c2c3-4561-874c-2f573a5409f2, checks=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, origin=ecb18413-cb3e-437e-9444-3cdd406ed67d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, terms file=528fe6f6-6d77-42d5-9086-017048d1448d

Problem: a map only agents wrote and verified carries no mark that the owner accepted it, and nothing tells a draft from a finished map.
Standard: sign-off is the accountable person accepting a reviewed artefact, recorded once with a hash of what was accepted.

- A) Yes: approve takes a blame file, checks it, and writes one ledger event over its hash, and a blame file that differs from its latest approval is a draft.
  Why: one command and one event, the same as a plan or the terms file.
  Costs: every pass by an agent leaves the file a draft until the owner approves again.
- B) No: agents write and verify it, and the commit hook checks only its shape.
  Why: no owner step.
  Costs: nothing records that anyone but the agent accepted the map.
- C) Yes, row by row: the owner approves each row's destination.
  Why: every disposition is the owner's.
  Costs: thousands of owner acts for one large origin.

**Answer:** A

### Does check hold a blame file to its origin?
depends: 34f0202a-0be4-4049-94de-d3886f4f8ffc=A|B, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, ecb18413-cb3e-437e-9444-3cdd406ed67d
id: 5fddeeb5-cf54-4550-a701-dd9e4a6a81d6
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, blame file=108053a8-c2c3-4561-874c-2f573a5409f2, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, current=e2533580-1486-4dc9-8697-e6c53605fe72, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, origin=ecb18413-cb3e-437e-9444-3cdd406ed67d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d

Problem: an origin edited after its map was approved leaves rows pointing at lines that no longer say what they did, and nothing notices.
Standard: a derived artefact records a checksum of its input, and a mismatch marks the artefact stale.

- A) Yes: check compares the origin at its path with the blob identifier in the blame file, and a mismatch fails until the agent updates the rows and the owner approves again.
  Why: a stale map never passes as current.
  Costs: every edit to an origin, even a typo, costs an owner approval.
- B) Yes, as a report: check names the mismatch and passes.
  Why: an origin can change freely while its map is being written.
  Costs: a stale map passes the hook.
- C) No: the blob identifier is written once and never compared.
  Why: the simplest file.
  Costs: the map drifts from the origin and nothing says so.

**Answer:** A

### Does an origin leave the repository while its blame file has an open row?
depends: 38216d39-b064-4927-ab60-951a3c89e388=A|B, e879fe72-ec07-48cd-9902-2b158aaae726=A|C, ecb18413-cb3e-437e-9444-3cdd406ed67d
id: 4bb5ef1b-da16-4206-a7d2-c7380c36a4cc
terms: answered=b36113a1-1384-4e4c-a798-8399f4181366, blame file=108053a8-c2c3-4561-874c-2f573a5409f2, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, origin=ecb18413-cb3e-437e-9444-3cdd406ed67d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, retired=c584a68d-e54a-4ad2-8a6b-9d64ed098ae5

Problem: an origin deleted before every question its text went to is answered leaves open questions with no text behind them.
Standard: a source is retired only after its replacement is current, and cut-over follows verification.

- A) No: check fails when an origin is missing while its blame file is unapproved or names a question that is not answered, and the origin may go once every such row is closed.
  Why: the text stays until every decision it fed is current.
  Costs: one open question keeps a whole origin in the working tree.
- B) Yes: an origin may leave at any time, and its rows keep the blob identifier.
  Why: the text is in git history whenever it is wanted.
  Costs: an open question's text is found only by digging in git.
- C) No, never: the origin stays as long as its blame file exists.
  Why: the map and the text always sit together.
  Costs: migrated text never leaves the working tree.

**Answer:** A

### Does the tool stop while a blame file waits for approval?
depends: e879fe72-ec07-48cd-9902-2b158aaae726=A|C, 108053a8-c2c3-4561-874c-2f573a5409f2, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de
id: 9c3cc17e-33f4-4855-b761-f8f82381cdca
terms: agents=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, blame file=108053a8-c2c3-4561-874c-2f573a5409f2, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, terms file=528fe6f6-6d77-42d5-9086-017048d1448d

Problem: agents edit a blame file many times before the owner approves it, and a draft that stopped the tool would halt every other question in the repository each time.
Standard: work in progress is visible to everyone and blocks nothing that does not depend on it.

- A) No: check reports a blame file awaiting approval as a count and passes, and ask and answer run.
  Why: passes over a map do not stop the rest of the repository.
  Costs: a draft map can sit unapproved for a long time.
- B) Yes: ask, answer and check stop while a blame file differs from its latest approval, as for the terms file.
  Why: one rule for every file the owner approves.
  Costs: every pass over the map halts every other question until the owner approves.
- C) Only its own questions: ask and answer stop the questions the draft's rows name.
  Why: the stop reaches only what the draft touches.
  Costs: a second stop rule to build and to learn.

**Answer:** A

### Does show print a blame file?
depends: 38216d39-b064-4927-ab60-951a3c89e388=*, 108053a8-c2c3-4561-874c-2f573a5409f2
id: 7b05f8df-09a5-404e-a0f0-3dd8824d1d15
terms: approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, blame file=108053a8-c2c3-4561-874c-2f573a5409f2, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, origin=ecb18413-cb3e-437e-9444-3cdd406ed67d, origins=ecb18413-cb3e-437e-9444-3cdd406ed67d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: with rows in a file and text in git, nobody sees at a glance what an origin became or what was dropped.
Standard: a reconciliation report gives totals per disposition and lists the exceptions.

- A) Yes: show prints each blame file with its row count per destination, every dropped row with its reason, and every question row with that question's state, and prints under each decision the origins that fed it.
  Why: the owner reads a summary at approval, and a decision shows where it came from.
  Costs: show grows a second section.
- B) Yes, the rows as written.
  Why: nothing is summarised away.
  Costs: thousands of lines for a large origin.
- C) No: the file is read by hand.
  Why: show stays about decisions.
  Costs: the summary the owner approves on does not exist.

**Answer:** A
