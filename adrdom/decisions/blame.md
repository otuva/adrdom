# Blame

How the text of an origin enters the records so that every line of it is accounted for: a blame file under adrdom/blame holds, for one origin, its line ranges and whether each holds a decision, and a question names the ranges it came from. Git keeps the text. Agents write the blame file, the owner approves it, and planning follows on its own.

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

### What does a blame file hold of its origin?
depends: 108053a8-c2c3-4561-874c-2f573a5409f2, ecb18413-cb3e-437e-9444-3cdd406ed67d, e877e4a9-2caf-4fe6-9f82-5476a2abb00b, e879fe72-ec07-48cd-9902-2b158aaae726, 5c399a14-1e5c-48eb-b502-b57524c01c24
supersedes: 34f0202a-0be4-4049-94de-d3886f4f8ffc, 38216d39-b064-4927-ab60-951a3c89e388, 5fddeeb5-cf54-4550-a701-dd9e4a6a81d6, 4bb5ef1b-da16-4206-a7d2-c7380c36a4cc, dce141e9-7d2e-4b70-b317-9f3d70bf5a69
unrelated: bceceda8-133a-4c9b-b439-b5c1a4414a02, 05141f49-d9d2-407c-b60c-ebb195a05864, f5c242b6-e2f6-45a3-8c61-c743f3ad794f, 672201dd-8f22-4dc3-8b21-a73eb7606e83, ee27fd4c-89b7-411f-a3e4-2535c49b6d1b, 6f6e7f4a-d733-4f3a-99f9-6a47fec5db70, 0c3d498a-c540-47f3-8384-050fc044c109, 110f471d-ac4b-4cbb-aa7b-3f74ac442d78, 796a2544-43e5-43db-b5d3-8a4e737ec9c6
id: d2851ea1-fcb6-476a-b321-c813a594d5e5
terms: approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, blame file=108053a8-c2c3-4561-874c-2f573a5409f2, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, option=43778946-d44c-4acf-a766-0a4915fce199, origin=ecb18413-cb3e-437e-9444-3cdd406ed67d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, term=f590059d-b681-40ec-8a1c-c8916d71f6bb, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: the records need, before any question is written, an approved account of which lines of an origin hold a decision.
Standard: an intermediate representation carries one classification per unit and no reference to what is built from it later.

- A) Path, blob identifier, an identifier line approve writes, one row per line range saying decision or detail, no text copied.
  Why: decision marks a range the owner settles by picking an option, detail every other range; the file closes before planning.
  Costs: whether a decision is in the records, or contradicts one, is found in planning, not in the row.
- B) Path, blob identifier, and one row per line range naming where it went: question, term, topic file or a drop.
  Why: every row resolves to a record the tool can find.
  Costs: a row needs its question first, so the file waits for planning and the owner approves last.
- C) Path, blob identifier, identifier line, one row per range saying decision or detail and, for a decision, its heading.
  Why: the file carries both the classification and the link.
  Costs: the approved file changes after planning, and the owner approves it twice.

**Answer:** A

### What does a question carry of the blame file it came from?
depends: 108053a8-c2c3-4561-874c-2f573a5409f2, 277a8338-c57a-43cc-8b28-6b42e9347f6a, e877e4a9-2caf-4fe6-9f82-5476a2abb00b, d2851ea1-fcb6-476a-b321-c813a594d5e5=A, e879fe72-ec07-48cd-9902-2b158aaae726, 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f, ecb18413-cb3e-437e-9444-3cdd406ed67d
unrelated: 5fddeeb5-cf54-4550-a701-dd9e4a6a81d6, 4bb5ef1b-da16-4206-a7d2-c7380c36a4cc, 34f0202a-0be4-4049-94de-d3886f4f8ffc, dce141e9-7d2e-4b70-b317-9f3d70bf5a69, 946263aa-519a-47aa-b743-5cdcf52f5d35, 38216d39-b064-4927-ab60-951a3c89e388, 7d47885f-3a45-45f9-a092-4314301f0d7b, 292a10b8-c287-4d71-b2bd-840da0a6de61, 5c399a14-1e5c-48eb-b502-b57524c01c24, bf6697e3-8832-4b90-b036-8fecedb55406, 99619e27-19b6-47ef-83b9-c79ac30ee2f0, f5c242b6-e2f6-45a3-8c61-c743f3ad794f, ee27fd4c-89b7-411f-a3e4-2535c49b6d1b, 6f6e7f4a-d733-4f3a-99f9-6a47fec5db70, 0c3d498a-c540-47f3-8384-050fc044c109, c10c3a49-e993-4c9f-a46b-4a1ba3646dcf
id: 672201dd-8f22-4dc3-8b21-a73eb7606e83
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, blame file=108053a8-c2c3-4561-874c-2f573a5409f2, blame line=108053a8-c2c3-4561-874c-2f573a5409f2, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, origin=ecb18413-cb3e-437e-9444-3cdd406ed67d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: the records need to read, from a question, which lines of which origin it came from, without a change to the approved blame file.
Standard: traceability runs from the derived artifact to its source by a stable identifier and a location.

- A) One blame line naming each blame file by identifier and the rows, by their ranges, the question itself came from.
  Why: the agent writes the file's name, approve turns it into the identifier; a question from no origin carries none.
  Costs: a line under the heading, approved with the plan and judged then, like a depends entry; never after.
- B) Nothing: the blame file's rows name the questions.
  Why: one file holds the whole map.
  Costs: the approved blame file changes after every plan, and the owner approves it again.
- C) Nothing on either side: the ledger records which blame file was open when the question was asked.
  Why: no line to write.
  Costs: a question from two files, or a file with many questions, cannot be distinguished.

**Answer:** A

### What does check do with a decision row that no question names?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, d2851ea1-fcb6-476a-b321-c813a594d5e5=A, 672201dd-8f22-4dc3-8b21-a73eb7606e83=A, 36dfbeaa-698d-4a57-b3b0-d425348e14a5
unrelated: 3fc35d4f-90f3-4425-9dac-951a89631fdf, 3460d220-e523-448b-aeff-baa8051b7d06, c10c3a49-e993-4c9f-a46b-4a1ba3646dcf, 23d55c81-2224-487c-a579-298915957a79, 110f471d-ac4b-4cbb-aa7b-3f74ac442d78, 751ef30c-6a2f-4936-bf9b-b9d487146e53, c84a4bd9-fc2d-44f0-b333-172978f61544, bf6697e3-8832-4b90-b036-8fecedb55406, 4319a653-6901-446c-94ab-6420ef472134, 05141f49-d9d2-407c-b60c-ebb195a05864, c3127af2-4c96-48d5-a164-bac49a9a09ec, 4bb5ef1b-da16-4206-a7d2-c7380c36a4cc, 5fddeeb5-cf54-4550-a701-dd9e4a6a81d6, 6f6e7f4a-d733-4f3a-99f9-6a47fec5db70, 0c3d498a-c540-47f3-8384-050fc044c109, 5c399a14-1e5c-48eb-b502-b57524c01c24
id: ee27fd4c-89b7-411f-a3e4-2535c49b6d1b
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, blame file=108053a8-c2c3-4561-874c-2f573a5409f2, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, finding=ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, origin=ecb18413-cb3e-437e-9444-3cdd406ed67d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: the records need to show which decision rows reached a question without making planning answer to the blame file.
Standard: traceability is reported as coverage, a count the reviewer reads, not a gate on the build.

- A) Nothing: show of the blame file counts, under a flag, its decision rows with a question and those with none.
  Why: a row's value is a first reading that planning may merge, drop or overturn; the owner reads the count.
  Costs: a decision row forgotten in planning stops nothing.
- B) Reports a finding for each such row.
  Why: every decision row ends in a question or a finding.
  Costs: a row planning rightly merged or dropped as a copy is a finding until someone rewrites the blame file.
- C) Nothing, and nothing is counted.
  Why: the blame file and the plan never meet.
  Costs: whether an origin's decisions reached the records is read by hand.

**Answer:** A

### What does check do when an origin differs from its blame file's blob identifier?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, ecb18413-cb3e-437e-9444-3cdd406ed67d, e877e4a9-2caf-4fe6-9f82-5476a2abb00b, d2851ea1-fcb6-476a-b321-c813a594d5e5=*, 05141f49-d9d2-407c-b60c-ebb195a05864, e879fe72-ec07-48cd-9902-2b158aaae726
supersedes: 5fddeeb5-cf54-4550-a701-dd9e4a6a81d6
unrelated: 34f0202a-0be4-4049-94de-d3886f4f8ffc, 4bb5ef1b-da16-4206-a7d2-c7380c36a4cc, bf6697e3-8832-4b90-b036-8fecedb55406, 6ef9eca6-fd8b-4211-aae7-00b37a2a9245, c10c3a49-e993-4c9f-a46b-4a1ba3646dcf, 26c5352f-3f82-4376-9dc2-8ccc0074d080, 912f4f3b-1213-435a-af0d-7617017fcff4, 76c6adc6-c018-4df4-b49a-ddcd0a03a48c, 5c399a14-1e5c-48eb-b502-b57524c01c24, 672201dd-8f22-4dc3-8b21-a73eb7606e83, ee27fd4c-89b7-411f-a3e4-2535c49b6d1b, 0c3d498a-c540-47f3-8384-050fc044c109
id: 6f6e7f4a-d733-4f3a-99f9-6a47fec5db70
terms: approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, blame file=108053a8-c2c3-4561-874c-2f573a5409f2, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, checked=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, origin=ecb18413-cb3e-437e-9444-3cdd406ed67d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d

Problem: the records need a blame file to hold to the text it was approved against.
Standard: a derived artifact records a content hash of its source and is checked against it.

- A) Fails on an approved blame file, until the rows and blob identifier are rewritten and the owner approves again.
  Why: a stale blame file never passes as approved; on a file awaiting approval the fault is a note.
  Costs: one edited line of the origin costs an owner approval.
- B) Reports the mismatch and passes.
  Why: an origin changes freely while its blame file is written.
  Costs: a stale blame file passes the hook.
- C) Nothing: the blob identifier is written once and never compared.
  Why: the simplest file.
  Costs: the blame file drifts from its origin and nothing says so.

**Answer:** A

### What does check do when an origin is absent?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, ecb18413-cb3e-437e-9444-3cdd406ed67d, d2851ea1-fcb6-476a-b321-c813a594d5e5=A, 6f6e7f4a-d733-4f3a-99f9-6a47fec5db70=*, 05141f49-d9d2-407c-b60c-ebb195a05864, e879fe72-ec07-48cd-9902-2b158aaae726
supersedes: 4bb5ef1b-da16-4206-a7d2-c7380c36a4cc
unrelated: 5fddeeb5-cf54-4550-a701-dd9e4a6a81d6, c10c3a49-e993-4c9f-a46b-4a1ba3646dcf, fe14991d-9715-4ae0-9a23-d314a950d3b9, 751ef30c-6a2f-4936-bf9b-b9d487146e53, 3fc35d4f-90f3-4425-9dac-951a89631fdf, bf6697e3-8832-4b90-b036-8fecedb55406, e3200081-9995-4bbe-983f-2f430dc341e7, 912f4f3b-1213-435a-af0d-7617017fcff4, 34f0202a-0be4-4049-94de-d3886f4f8ffc, 5c399a14-1e5c-48eb-b502-b57524c01c24, 34e81f57-9b3c-4dc4-a2ba-ca227523177d, 76c6adc6-c018-4df4-b49a-ddcd0a03a48c, 6108b8b2-911e-443b-abe0-4c6f8b407384, 0d977e39-a657-4596-8eca-8ceb335f3081, 672201dd-8f22-4dc3-8b21-a73eb7606e83, ee27fd4c-89b7-411f-a3e4-2535c49b6d1b
id: 0c3d498a-c540-47f3-8384-050fc044c109
terms: answered=b36113a1-1384-4e4c-a798-8399f4181366, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, blame file=108053a8-c2c3-4561-874c-2f573a5409f2, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, finding=ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, origin=ecb18413-cb3e-437e-9444-3cdd406ed67d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, retired=c584a68d-e54a-4ad2-8a6b-9d64ed098ae5, retires=c584a68d-e54a-4ad2-8a6b-9d64ed098ae5

Problem: the records need to say when an origin's text may leave the working tree, and who takes it out.
Standard: a source is retired after its replacement is verified, and the owner of the source retires it, not the tool.

- A) Nothing while the blame file is approved; a finding while it is not.
  Why: the approved blame file is the threshold; the owner deletes the origin and git keeps the text.
  Costs: an open question's text is read from git.
- B) A finding until every question the blame file's rows fed is answered.
  Why: the text stays until every decision it fed is current.
  Costs: rows name no question, so the rule has nothing to hold, and one open question keeps a whole origin.
- C) A finding always.
  Why: the blame file and the text always sit together.
  Costs: migrated text never leaves the working tree.

**Answer:** A
