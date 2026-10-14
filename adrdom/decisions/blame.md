# Blame

How the text of an origin enters the records so that every line of it is accounted for: a blame file under adrdom/blame holds, for one origin, its line ranges and whether each holds a decision, and a question names the ranges it came from. Git keeps the text. Agents write the blame file, the owner approves it, and planning follows on its own.

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

### What does a question carry of its blame file?
depends: 108053a8-c2c3-4561-874c-2f573a5409f2, 277a8338-c57a-43cc-8b28-6b42e9347f6a, ecb18413-cb3e-437e-9444-3cdd406ed67d, d2851ea1-fcb6-476a-b321-c813a594d5e5
supersedes: 672201dd-8f22-4dc3-8b21-a73eb7606e83, ee27fd4c-89b7-411f-a3e4-2535c49b6d1b
unrelated: 946263aa-519a-47aa-b743-5cdcf52f5d35, 3ce82f34-a4fc-4918-bb9b-65a0d81a6853, 7d47885f-3a45-45f9-a092-4314301f0d7b, 6f6e7f4a-d733-4f3a-99f9-6a47fec5db70, 0c3d498a-c540-47f3-8384-050fc044c109, aef1b384-2b33-47a2-92a8-ec7f8fb258ef, 5c399a14-1e5c-48eb-b502-b57524c01c24, bf6697e3-8832-4b90-b036-8fecedb55406, c10c3a49-e993-4c9f-a46b-4a1ba3646dcf, 99619e27-19b6-47ef-83b9-c79ac30ee2f0, e879fe72-ec07-48cd-9902-2b158aaae726, a469ecb9-a4f1-4a2a-89c5-7378cc1138b4, f5c242b6-e2f6-45a3-8c61-c743f3ad794f
id: 69708a07-469c-406b-9e25-63da1605b432
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, blame file=108053a8-c2c3-4561-874c-2f573a5409f2, blame line=108053a8-c2c3-4561-874c-2f573a5409f2, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, origin=ecb18413-cb3e-437e-9444-3cdd406ed67d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: the records need to read, from a question, which lines of which origin it came from, without a change to the approved blame file.
Standard: traceability runs from the derived artifact to its source by a stable identifier and a location.

- A) One blame line naming each blame file by identifier and the rows, by their ranges, the question itself came from.
  Why: the agent writes the file's name and the identifier rule turns it; a question from no origin carries none.
  Costs: a line under the heading, approved with the plan and judged then, like a depends entry; never after.
- B) Nothing: the blame file's rows name the questions.
  Why: one file holds the whole map.
  Costs: the approved blame file changes after every plan, and the owner approves it again.
- C) Nothing on either side: the ledger records which blame file was open when the question was asked.
  Why: no line to write.
  Costs: a question from two files, or a file with many questions, cannot be distinguished.

**Answer:** A

### What does check do with a decision row no question names?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, 108053a8-c2c3-4561-874c-2f573a5409f2, d2851ea1-fcb6-476a-b321-c813a594d5e5, 69708a07-469c-406b-9e25-63da1605b432=A
supersedes: ee27fd4c-89b7-411f-a3e4-2535c49b6d1b
unrelated: de1c5329-e448-45eb-b14c-88b8654447a4, c10c3a49-e993-4c9f-a46b-4a1ba3646dcf, 3fc35d4f-90f3-4425-9dac-951a89631fdf, 1259323f-cdec-4101-9d47-a165a286089f, 4319a653-6901-446c-94ab-6420ef472134, 0c3d498a-c540-47f3-8384-050fc044c109, 2381d759-f85f-484e-9031-fadbe6ea045a, 3460d220-e523-448b-aeff-baa8051b7d06, c84a4bd9-fc2d-44f0-b333-172978f61544, bf6697e3-8832-4b90-b036-8fecedb55406, c3127af2-4c96-48d5-a164-bac49a9a09ec, 6f6e7f4a-d733-4f3a-99f9-6a47fec5db70, 05141f49-d9d2-407c-b60c-ebb195a05864
id: a469ecb9-a4f1-4a2a-89c5-7378cc1138b4
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
