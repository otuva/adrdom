# Change

How a decision leaves force, how the owner turns a question away, and what happens to a
question once it no longer holds a place in the plan.

### Does one decision supersede several?
depends: dad5273c-afe2-4ab1-a451-e2a5d4d9a26d, 2bae531b-c4df-48a0-89a0-3d52726741ea
id: f4f667ad-8714-4e65-897b-c7002e423c28
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, answered=b36113a1-1384-4e4c-a798-8399f4181366, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, supersede=2bae531b-c4df-48a0-89a0-3d52726741ea, superseded=2bae531b-c4df-48a0-89a0-3d52726741ea, supersedes=2bae531b-c4df-48a0-89a0-3d52726741ea, topic=292a10b8-c287-4d71-b2bd-840da0a6de61

Problem: when a whole topic's premise turns out wrong, replacing its decisions one at a time costs one question and one answer per decision, and splitting one decision into several has no route.
Standard: architecture decision records let a new record supersede an earlier one by reference; most tools allow one link each way.

- A) Yes, in both directions: a supersedes line may name several decisions, and several newer decisions may name the same older one; each target must be current when the question is asked, and the older one leaves force when the first replacement is answered.
  Why: a wrong premise costs one question, and each part of a split stays its own decision.
  Costs: one answer can end many decisions at once, so the chosen sentence has to say what holds now for all of them.
- B) One to one: a supersedes line names one decision, and a decision is superseded once.
  Why: every link reads alone.
  Costs: replacing a topic of ten decisions is ten questions, and a split has no route.

**Answer:** A

### Does the owner answer a question with a refusal?
depends: b36113a1-1384-4e4c-a798-8399f4181366, 0942e445-0163-423e-8ba7-84dd8ac07045, 277a8338-c57a-43cc-8b28-6b42e9347f6a
id: b782664f-7465-4e1e-99e0-13f6dd2de774
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, answers=b36113a1-1384-4e4c-a798-8399f4181366, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, options=43778946-d44c-4acf-a766-0a4915fce199, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, refusal=0942e445-0163-423e-8ba7-84dd8ac07045, refused=0942e445-0163-423e-8ba7-84dd8ac07045, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c

Problem: a question whose heading assumes something nothing has settled, or whose options are all wrong, has no honest letter; without another move the owner picks a wrong letter or the plan is rolled back.
Standard: review tools let a reviewer reject a change without editing it.

- A) Yes: the owner answers with a letter or a refusal; a refused question closes with no decision and no test, its identifier never returns, and a replacement is a new question.
  Why: one owner move closes a bad question, and the work goes forward from where it is.
  Costs: a second kind of answer in the ledger and on the page.
- B) No: a letter only; a bad question is struck from the plan, the plan approved again, and the tier asked again.
  Why: one kind of answer.
  Costs: three owner moves per bad question.
- C) Yes, and the refused question stays open for the agent to rewrite under the same identifier.
  Why: the identifier and its place in the plan survive.
  Costs: an asked question's text moves under its identifier after the owner read it.

**Answer:** A

### Does approve withdraw an asked question struck from the plan?
depends: da59b448-1afb-4469-b40e-69eda8f253c2, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, e4a2ef0c-3354-4c9b-a8d9-9e204e544d85
id: 21ae8ae0-6884-4d3c-8464-024a8a5aae2d
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answered=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approving=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6, withdraw=e4a2ef0c-3354-4c9b-a8d9-9e204e544d85, withdrawn=e4a2ef0c-3354-4c9b-a8d9-9e204e544d85, withdraws=e4a2ef0c-3354-4c9b-a8d9-9e204e544d85

Problem: a heading struck after its question was asked leaves an identifier with an asked event and no section; something has to write the event that closes it, and it has to be clear whose act that was.
Standard: issue trackers record a closed item as closed by the person who closed it and keep its history.

- A) Yes: when the owner approves a plan that no longer holds an asked question, approve writes a withdrawn event for it; approve stops when an answered question is missing from every topic file; a withdrawn identifier never returns.
  Why: the strike and the withdrawal are one act the owner performs by approving.
  Costs: an asked question struck from the plan closes only at the owner's next approve.
- B) No: the next ask writes the withdrawn event.
  Why: nothing new in approve.
  Costs: a command the agent runs closes the owner's question, and check fails between the strike and the next ask.
- C) No: a separate owner command withdraws a question before its heading may leave the file.
  Why: an explicit act.
  Costs: a second owner move for one change.

**Answer:** A

### Does a superseded question leave its topic file?
depends: dad5273c-afe2-4ab1-a451-e2a5d4d9a26d, 2bae531b-c4df-48a0-89a0-3d52726741ea, 08193e75-dd89-4857-8e9f-b19e1a9e75e6
id: 1b181f10-97f4-4bbe-8939-d5842192126c
terms: answered=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, superseded=2bae531b-c4df-48a0-89a0-3d52726741ea, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6, topic files=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: a decision leaves force only when superseded, but if its section must stay forever, topic files fill with text no current decision needs, and a topic file can never be removed.
Standard: version control keeps old text; a working document drops what is replaced.

- A) Yes: once superseded, its section may leave the file as a plan change the owner approves, and approve allows that only for superseded questions; a topic file with no current or open question left may be deleted the same way; the old text stays in git history and its hash in the ledger.
  Why: the file holds what is in force and what is open, and removal stays the owner's act.
  Costs: reading an old decision's text means git history.
- B) No: it stays in the file forever.
  Why: everything ever answered is readable in the working file.
  Costs: sections show never prints pile up, and a topic file can never be removed.
- C) It moves to an archive file the tool owns.
  Why: out of the way and still in the tree.
  Costs: a second copy of what git keeps.

**Answer:** A

### Does a refused question stay in its topic file?
depends: b782664f-7465-4e1e-99e0-13f6dd2de774=A, 08193e75-dd89-4857-8e9f-b19e1a9e75e6, 94677ed8-eaf6-46e2-bc71-7392fa0805bf
id: 6b3d8e59-e145-49e1-860d-a79d753d745c
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, refusal=0942e445-0163-423e-8ba7-84dd8ac07045, refused=0942e445-0163-423e-8ba7-84dd8ac07045, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: a refused question has a body, a ledger event and no decision; if it leaves at once the file loses what was turned away, and if it never leaves the file carries it for good.
Standard: issue trackers keep a closed item visible until a deliberate act archives it.

- A) Yes, until the next plan change: the section stays, marked refused, and when the owner approves a plan that drops it, the section is gone.
  Why: the file shows what was turned away until the plan moves on, and removal stays the owner's act.
  Costs: a refused section sits in the file between the refusal and the next plan change.
- B) Yes, for good: no plan change may drop a refused section.
  Why: the file is a full record of what was asked.
  Costs: dead sections pile up with no way out.
- C) No: the agent removes the section at its next commit, and the ledger event is the only record.
  Why: the file holds only live questions.
  Costs: an agent edit removes an asked heading from an approved plan, and the plan fails until the owner approves again.

**Answer:** A

### Does a refused question hold the tiers above it?
depends: b782664f-7465-4e1e-99e0-13f6dd2de774=A, f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c
id: 506b3c6d-3d0a-4a02-98de-3b8a573d2fe9
terms: answered=b36113a1-1384-4e4c-a798-8399f4181366, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, asking=3c8d4c9c-abca-4faa-b7c1-aae39951376c, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, refusal=0942e445-0163-423e-8ba7-84dd8ac07045, refused=0942e445-0163-423e-8ba7-84dd8ac07045, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, tiers=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, topic=292a10b8-c287-4d71-b2bd-840da0a6de61

Problem: a question above a refused one was written on the assumption the refused one would be answered; asking it anyway puts a question before the owner whose ground is gone.
Standard: dependency-ordered work stops at the unmet prerequisite and names it; nothing proceeds on a placeholder.

- A) Yes: a refused question counts as unanswered for its tier, so the next tier waits until the owner approves a plan change that drops or replaces it.
  Why: nothing is asked on ground the owner turned away, and the refusal sends the plan back through the one gate.
  Costs: one refusal holds every tier above it until a plan change.
- B) No: a refused question counts as settled, and the next tier opens.
  Why: a refusal never stalls the topic.
  Costs: questions standing on the refused one reach the owner with their ground gone.

**Answer:** A

### Does a supersedes line name every current decision standing on its target?
depends: f4f667ad-8714-4e65-897b-c7002e423c28=A, 2bae531b-c4df-48a0-89a0-3d52726741ea, aef1b384-2b33-47a2-92a8-ec7f8fb258ef, e2533580-1486-4dc9-8697-e6c53605fe72
id: 796a2544-43e5-43db-b5d3-8a4e737ec9c6
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, refuses=0942e445-0163-423e-8ba7-84dd8ac07045, superseded=2bae531b-c4df-48a0-89a0-3d52726741ea, supersedes=2bae531b-c4df-48a0-89a0-3d52726741ea, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: a decision leaves force while decisions standing on it stay current, and nothing says so.
Standard: referential integrity: a record that others reference is not removed while the references stand.

- A) Yes: the line names every current decision standing on a named target, in every topic file and through the decisions standing on those, and approve refuses a line that leaves one out and names the missing ones.
  Why: no decision loses its ground unnoticed; the plan holds the whole tree that changes, and the owner reads it as one.
  Costs: approve reads every topic file to find the tree; a dependent in another file leaves force with the root, and its replacement is written in the root's file or after the root's answer.
- B) No: the line names what the agent chooses, and check reports every current decision left standing on a superseded one until a question supersedes it.
  Why: a small change stays small; the owner is not made to decide again what would not move.
  Costs: between the answer and the follow-up questions, current decisions stand on nothing; the report is the only guard, and the follow-up rests on the agent.

**Answer:** A

### Does a tier hold its cap by the count after its supersedes lines take effect?
depends: f4f667ad-8714-4e65-897b-c7002e423c28=A, 15d82e53-e344-4347-b639-3bf6eb9ef1b4, f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, 2bae531b-c4df-48a0-89a0-3d52726741ea
id: f416062c-211f-4d20-a6bf-23a561b5f59b
terms: answered=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, refusal=0942e445-0163-423e-8ba7-84dd8ac07045, refused=0942e445-0163-423e-8ba7-84dd8ac07045, superseded=2bae531b-c4df-48a0-89a0-3d52726741ea, supersedes=2bae531b-c4df-48a0-89a0-3d52726741ea, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6, withdrawn=e4a2ef0c-3354-4c9b-a8d9-9e204e544d85

Problem: a question replacing a decision in a full tier is refused on the cap, the old and the new counted together, so a decision in a full tier cannot be replaced in its own file.
Standard: a limit on a set is judged on the set as the change leaves it.

- A) Yes: a decision named on the supersedes line of a question not yet answered is left out of its tier's count, so the cap holds by what the tier holds once the replacement is answered.
  Why: a replacement in a full tier is a legal move, and the file that holds a subject keeps holding it.
  Costs: approve and ask count from the supersedes lines of open questions; a refused or withdrawn replacement puts the old back in the count, and two replacements of one decision count as two.
- B) No: every question that is not refused, superseded or withdrawn counts, old and new alike, and a replacement in a full tier goes to another topic file.
  Why: one count, read straight off the file.
  Costs: a full tier takes no replacement in its own file, so the replacement lands in a file away from the subject it changes, and the refusal names moves that do not fit a replacement.

**Answer:** A

### What may leave a topic file by a plan change?
depends: 2bae531b-c4df-48a0-89a0-3d52726741ea, 0942e445-0163-423e-8ba7-84dd8ac07045, 08193e75-dd89-4857-8e9f-b19e1a9e75e6, dad5273c-afe2-4ab1-a451-e2a5d4d9a26d, 6b3d8e59-e145-49e1-860d-a79d753d745c, 94677ed8-eaf6-46e2-bc71-7392fa0805bf
supersedes: 1b181f10-97f4-4bbe-8939-d5842192126c
unrelated: d981f6b0-df0d-428f-8c0e-e220b1651fa1, 912f4f3b-1213-435a-af0d-7617017fcff4, 23f21671-e34c-44aa-8804-6163d975807f, aef1b384-2b33-47a2-92a8-ec7f8fb258ef, d3be71e4-ce7b-43b0-86a8-e86868521f99, 292a10b8-c287-4d71-b2bd-840da0a6de61, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, 4bb5ef1b-da16-4206-a7d2-c7380c36a4cc, 4f04bae4-98e3-48eb-9403-3257268e2d87, 75075c78-fe3c-4fd8-8a33-29a5913d9d34, 05af74a6-36be-4923-b66f-1467b323816a
id: 79da6500-2246-4a80-baf1-0eaf7cad49ee
terms: answered=b36113a1-1384-4e4c-a798-8399f4181366, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, refused=0942e445-0163-423e-8ba7-84dd8ac07045, superseded=2bae531b-c4df-48a0-89a0-3d52726741ea, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6, topic files=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: a decision leaves force only when superseded or refused, but if its section must stay forever, topic files fill with text no current decision needs, and a topic file can never be removed.
Standard: version control keeps old text; a working document drops what is replaced.

- A) The section of a superseded or refused question, and a file with no current or open question left.
  Why: the file holds what is in force and what is open, and removal stays the owner's act.
  Costs: reading an old decision's text means git history.
- B) Nothing: every section stays in its file for good.
  Why: everything ever answered is readable in the working file.
  Costs: sections show never prints pile up, and a topic file can never be removed.
- C) A superseded or refused section, into an archive file the tool owns.
  Why: out of the way and still in the tree.
  Costs: a second copy of what git keeps.

**Answer:** A

### What strikes the section of a superseded question from its topic file?
depends: 08193e75-dd89-4857-8e9f-b19e1a9e75e6, b36113a1-1384-4e4c-a798-8399f4181366, 2bae531b-c4df-48a0-89a0-3d52726741ea, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, 277a8338-c57a-43cc-8b28-6b42e9347f6a, 79da6500-2246-4a80-baf1-0eaf7cad49ee
unrelated: 6b3d8e59-e145-49e1-860d-a79d753d745c, f5c242b6-e2f6-45a3-8c61-c743f3ad794f, 9913568f-6b85-4d26-946b-c6e108f9aa51, 9afe335c-2a7a-4625-b115-f74d3f091cac, 23f21671-e34c-44aa-8804-6163d975807f, 912f4f3b-1213-435a-af0d-7617017fcff4, aef1b384-2b33-47a2-92a8-ec7f8fb258ef, 292a10b8-c287-4d71-b2bd-840da0a6de61, 7a1e7b81-0967-4ed2-8a8a-c7153056836f, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, 21ae8ae0-6884-4d3c-8464-024a8a5aae2d, 34e81f57-9b3c-4dc4-a2ba-ca227523177d, cd83e277-ebf0-4458-a77a-c526ab4cfeca, fcc5b6fe-9072-42ea-a25b-2be72e094bac, 7c0b9fe3-6038-4a23-93fe-88a31c7eaf40
id: b5ca161f-c88c-4f02-a9d9-1d406cf3a252
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, answered=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, superseded=2bae531b-c4df-48a0-89a0-3d52726741ea, supersedes=2bae531b-c4df-48a0-89a0-3d52726741ea, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6, topic files=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: a superseded section may leave by a plan change, and no command makes one, so topic files fill with sections show hides.
Standard: a working document drops what is replaced the moment it is replaced; version control keeps the old text.

- A) Answer: every run strikes each superseded section from every topic file, and check reports one still there.
  Why: the command that supersedes leaves no dead section behind, and the ones that piled up go at the next answer.
  Costs: answer writes topic files beyond the one answered; a file's approved hash is read without the struck sections.
- B) The agent, as a plan change the owner approves.
  Why: every edit of a topic file stays an edit the owner reads.
  Costs: nothing makes the change, so dead sections pile up until someone strikes them and the owner approves each file.
- C) Approve: it strikes each superseded section of the file it approves.
  Why: a file is cleaned by the owner's act, when the owner reads it anyway.
  Costs: a file with no plan change keeps its dead sections for good.

**Answer:** A
