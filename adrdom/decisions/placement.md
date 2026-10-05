# Placement

How a new question is tied to the decisions that exist: the steps of planning a topic file, the neighbours the tool brings, what the reader says about them, the line the agent writes for each, and what happens to a question whose answer set lacks the owner's answer.

### Does the agent write the bodies before approve?
depends: f90dbd5b-8fbe-4899-8588-f8762795c2e2, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, 94677ed8-eaf6-46e2-bc71-7392fa0805bf
supersedes: f01e20ae-00fc-4ec9-a25b-88a90dffc624, 258107eb-2a90-4ce9-a111-d1bba047d423
id: f5eeaacf-1c9f-478b-8a97-2df1aaf39458
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, answer sets=9d634996-475e-4ddc-8113-05c404ad9d5c, answers=b36113a1-1384-4e4c-a798-8399f4181366, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, options=43778946-d44c-4acf-a766-0a4915fce199, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, tiers=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c

Problem: a tie that shows only in the options is found after the plan is fixed, and fixing it is a plan change.
Standard: review a change whole, structure and content together, once.

- A) Yes.
  Why: the owner reads headings, lines and options once; a tie seen in the options moves the plan before approval.
  Costs: approval covers every body; bodies of upper tiers are written under declared answer sets before any answer.
- B) No.
  Why: approval stays small and early; a body follows the answers under it.
  Costs: a tie that shows only in the options is found after approval, and fixing it is a plan change.

**Answer:** A

### Does approve hash each question on its own?
depends: e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, 277a8338-c57a-43cc-8b28-6b42e9347f6a
id: 61e73795-63de-4669-b09f-48be96919325
terms: approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, question=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: one hash over the whole file means any section leaving or changing unsettles every other section's approval.
Standard: hash content by unit and derive the whole from the parts, so one part's change names itself.

- A) Yes.
  Why: a removed question takes its hash with it and nothing else moves; check sees which section changed.
  Costs: one approval row per question in the ledger; the file's hash is derived from the sorted question hashes.
- B) No.
  Why: one row and one comparison per file.
  Costs: a question removed after approval changes the file hash and the file waits for approval again.

**Answer:** A

### Does a depends entry on a question in its file carry an answer set?
depends: 9d634996-475e-4ddc-8113-05c404ad9d5c, aef1b384-2b33-47a2-92a8-ec7f8fb258ef, f5eeaacf-1c9f-478b-8a97-2df1aaf39458=*
id: 946263aa-519a-47aa-b743-5cdcf52f5d35
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, answer set=9d634996-475e-4ddc-8113-05c404ad9d5c, answered=b36113a1-1384-4e4c-a798-8399f4181366, answers=b36113a1-1384-4e4c-a798-8399f4181366, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c

Problem: an upper-tier question is written before the question under it is answered; without the letters it holds under, a moot question looks like a live one.
Standard: a conditional item names its condition.

- A) Yes.
  Why: the letters are written with the body, before approval; answer tells a moot question by them.
  Costs: approve stops a plan where a depends entry on a question in its file lacks its set.
- B) No.
  Why: nothing to write; every question in the plan is asked whatever the answers below it.
  Costs: a question built on an answer the owner did not give is asked and answered anyway.

**Answer:** A

### Does check fail a topic file awaiting approval?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, f5eeaacf-1c9f-478b-8a97-2df1aaf39458=*
id: 912f4f3b-1213-435a-af0d-7617017fcff4
terms: approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, refuse=0942e445-0163-423e-8ba7-84dd8ac07045, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: a plan with its bodies is written in git before the owner reads it; failing it blocks the draft, ignoring it hides it.
Standard: linters report pending items as counts and fail only what is wrong.

- A) No.
  Why: the draft is committed and read in git; the count keeps a stale plan visible.
  Costs: check counts the unapproved files on every run.
- B) Yes.
  Why: nothing unapproved enters git.
  Costs: the plan with its bodies is shown in conversation only, and the hooks refuse the draft commit.

**Answer:** A

### Does answer remove a question whose answer set lacks the answer?
depends: b36113a1-1384-4e4c-a798-8399f4181366, 946263aa-519a-47aa-b743-5cdcf52f5d35=A, 61e73795-63de-4669-b09f-48be96919325=A
supersedes: fa90692a-150e-4a89-91e2-1d575cb59d0f
id: 462e22c6-8b23-4061-9a2a-bbf8691fce60
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, answer set=9d634996-475e-4ddc-8113-05c404ad9d5c, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: a question whose set lacks the answer is dead, and it sits in the file until a new approval removes it; the owner approves twice for one answer.
Standard: a conditional item leaves with its condition; no second confirmation.

- A) Yes.
  Why: the owner's answer is the ruling; the question and those standing on it leave at once, no second approval.
  Costs: answer edits the topic file past its answer line; removed text stays in git history, its hash in a row.
- B) No.
  Why: answer writes only its answer line; the plan leaves the file to approve.
  Costs: a dead question waits in the file until the owner approves a plan without it; ask and check report it.

### What does answer write for a removed question?
depends: e18b82d8-5271-4385-bbab-2b89e5da278f, 462e22c6-8b23-4061-9a2a-bbf8691fce60=A
id: f5c242b6-e2f6-45a3-8c61-c743f3ad794f

Problem: a question leaves the file at the owner's answer; without a row the approved set and the file disagree and nothing says why.
Standard: an append-only log records every removal with its cause.

- A) A dropped row: the question's identifier, its hash from approval, the identifier and letter of the answer that dropped it.
  Why: check verifies the drop from the row: that answer was given and the set lacked that letter.
  Costs: one row per removed question, dependents included, each naming the question it stood on.
- B) A dropped row with the question's identifier and its hash from approval, no cause.
  Why: the shortest row that keeps the approved set whole.
  Costs: check sees the removal but cannot verify it was legal; the cause lives in git history only.

### What does the tool bring as neighbours for a new heading?
depends: daec9ffd-676f-4111-a359-2827bfd4527c, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, 607cb55f-c0fa-40af-b290-c17f3ba0f812
id: de8ef568-3a8d-42ba-bef7-d7fd467b27be
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, definition=607cb55f-c0fa-40af-b290-c17f3ba0f812, definitions=607cb55f-c0fa-40af-b290-c17f3ba0f812, embedding=fa0b0e24-b7cf-471b-94b6-6d167e8ad269, embedding model=a7b0b910-57f6-4c59-b904-b42f78db63f7, embeddings=fa0b0e24-b7cf-471b-94b6-6d167e8ad269, neighbour=daec9ffd-676f-4111-a359-2827bfd4527c, neighbours=daec9ffd-676f-4111-a359-2827bfd4527c, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, stands on=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, term=f590059d-b681-40ec-8a1c-c8916d71f6bb, terms file=528fe6f6-6d77-42d5-9086-017048d1448d

Problem: the agent ties a new question only to the decisions it remembers; the ones it repeats or stands on stay out of view.
Standard: nearest-neighbour retrieval over embeddings of the records, a fixed count per query.

- A) The ten nearest current decisions and term definitions, by embedding of the heading.
  Why: a repeated decision, a missing tie or a definition that states the answer comes into view before approval.
  Costs: ten is a cut, what falls past it is not seen; the embedding model must be on the machine.
- B) The ten nearest current decisions, by embedding of the heading.
  Why: fewer texts to rule on; definitions stay a matter of the terms file.
  Costs: a question whose answer a definition states passes, and two rules live in two places.
- C) Every current decision within a fixed distance of the heading's embedding.
  Why: the count follows the data; a crowded subject brings more, a lone one fewer.
  Costs: a distance is harder to set than a count and can bring none or dozens.

**Answer:** A

### Does git track a file the tool can get again without the owner?
depends: bf7da2dd-d8a7-41b8-9898-11cf68d0e833, a7b0b910-57f6-4c59-b904-b42f78db63f7
id: 0d977e39-a657-4596-8eca-8ceb335f3081
terms: check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, embedding index=bf7da2dd-d8a7-41b8-9898-11cf68d0e833, embedding model=a7b0b910-57f6-4c59-b904-b42f78db63f7, install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, plans=94677ed8-eaf6-46e2-bc71-7392fa0805bf

Problem: the embedding index and the embedding model are large, derived, and change with the records and the model.
Standard: generated and downloaded files stay out of version control and are rebuilt on each machine.

- A) No.
  Why: the repository holds records only; a clone rebuilds the index and fetches the model on first use.
  Costs: planning on a fresh clone starts with a download and a rebuild; install writes the ignore file, check holds it.
- B) Yes.
  Why: a clone plans at once, with no download and no rebuild.
  Costs: a hundred megabytes in git and an index that changes at every plan; merges of a binary file.

**Answer:** A

### What does the agent do to plan a topic file?
depends: f90dbd5b-8fbe-4899-8588-f8762795c2e2, 94677ed8-eaf6-46e2-bc71-7392fa0805bf, de8ef568-3a8d-42ba-bef7-d7fd467b27be=*
id: 46fc51ff-66b9-4841-a953-ae0d4c2fc424
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, asks=3c8d4c9c-abca-4faa-b7c1-aae39951376c, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, neighbour=daec9ffd-676f-4111-a359-2827bfd4527c, neighbours=daec9ffd-676f-4111-a359-2827bfd4527c, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: an agent that writes headings, ties and bodies in one pass sees the ties it remembers and no other; nothing orders the work.
Standard: a pipeline with a fixed order of steps, each step's output the next step's input.

- A) Writes headings, gets neighbours, writes lines, writes bodies, gets neighbours of the full questions, asks approval.
  Why: ties are written with the neighbours in view, bodies with the ties, and the full questions are read again.
  Costs: six steps and two tool runs per plan; a heading reworded after its neighbours came is read again.
- B) Writes headings, gets neighbours, writes lines and bodies, asks approval.
  Why: one tool run; the bodies are written with the neighbours in view.
  Costs: the neighbours a body brings and a heading does not are never seen.
- C) Writes headings, lines and bodies, then gets neighbours of the full questions, asks approval.
  Why: one tool run, over the fullest text.
  Costs: lines are written with no neighbour in view, and a heading that repeats a decision gets a body first.

**Answer:** A

### Does a planning step run without the record of the step before?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, 46fc51ff-66b9-4841-a953-ae0d4c2fc424=*
id: d3be71e4-ce7b-43b0-86a8-e86868521f99
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, checks=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, neighbours=daec9ffd-676f-4111-a359-2827bfd4527c, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, refused=0942e445-0163-423e-8ba7-84dd8ac07045, refuses=0942e445-0163-423e-8ba7-84dd8ac07045

Problem: the steps order the work only if the tool can see that each was done; an agent that writes everything at once and runs the tool last has skipped nothing the tool can name.
Standard: a pipeline stage refuses input that lacks the previous stage's output.

- A) No.
  Why: neighbours come for headings without bodies, lines are read before the full questions, approval reads records of the text.
  Costs: one command whose step follows the file's state; a file with bodies under some headings only is refused.
- B) Yes.
  Why: the agent runs the steps in any order; approval checks that every record exists.
  Costs: the order of thinking is not held; neighbours of a heading may come after its body was written.

### Does planning stop without the embedding model?
depends: a7b0b910-57f6-4c59-b904-b42f78db63f7, de8ef568-3a8d-42ba-bef7-d7fd467b27be=*
id: 21cdad59-4f0c-42a5-b4f3-6af946637df3
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, embedding model=a7b0b910-57f6-4c59-b904-b42f78db63f7, neighbour=daec9ffd-676f-4111-a359-2827bfd4527c, neighbours=daec9ffd-676f-4111-a359-2827bfd4527c, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, plans=94677ed8-eaf6-46e2-bc71-7392fa0805bf

Problem: the model is a file outside git; a machine without it brings no neighbour, and a plan made without neighbours has the holes this work closes.
Standard: a step whose input is missing stops and names it; no degraded mode.

- A) Yes.
  Why: one shape of plan; a plan with no neighbour rows is never approved by mistake.
  Costs: the agent fetches the model or plans on another machine; every other command runs without it.
- B) No.
  Why: planning runs anywhere; the neighbour step is skipped with a note.
  Costs: two shapes of plan, and approve cannot tell a skipped step from a done one.

**Answer:** A

### Does the tool bring the plan's own questions as neighbours of each other?
depends: 94677ed8-eaf6-46e2-bc71-7392fa0805bf, 62e50382-db1b-4649-9f26-7f44c379d6fe=A
id: 4838baa4-a5df-49e7-929c-ed63b5adaf4e

### Does every neighbour get a line under its heading?
depends: aef1b384-2b33-47a2-92a8-ec7f8fb258ef, 2bae531b-c4df-48a0-89a0-3d52726741ea, 46fc51ff-66b9-4841-a953-ae0d4c2fc424=*
id: c6451ecc-17a9-447e-a9c9-fe9b35ec789c
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, identifiers=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, neighbour=daec9ffd-676f-4111-a359-2827bfd4527c, reader=be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, supersedes=2bae531b-c4df-48a0-89a0-3d52726741ea

Problem: the agent writes the ties it sees; a neighbour it never ruled on looks the same as one it judged unrelated.
Standard: every item put in front of a reviewer gets an explicit disposition.

- A) Yes.
  Why: forgetting is impossible; what is left is a wrong ruling, and that stands under the agent's own line.
  Costs: an unrelated line joins depends and supersedes under the heading; approval turns its names to identifiers.
- B) No.
  Why: only the ties that hold are written; the file stays short.
  Costs: a neighbour left off a line is unseen by approve, and the reader's word on it meets no ruling.

**Answer:** A

### What does the reader say about a neighbour of a heading?
depends: be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, c6e0b7d6-cdec-46da-b2bc-cda218673868=A
id: 62e50382-db1b-4649-9f26-7f44c379d6fe

Problem: at the heading step the new side has no options and no answer; consistent and inconsistent need the rules of both sides.
Standard: a classifier gets only the labels its input can support.

- A) One of same, depends, unrelated, after one sentence of reason.
  Why: a heading a decision settles drops before its body is written; a missing tie is named early.
  Costs: ten reader calls per heading, repeated at the body step because the text changes.
- B) Nothing: the reader reads a question only with its body.
  Why: half the reader calls; the agent reads the ten neighbours itself at the heading step.
  Costs: a heading a decision settles gets a body before it drops; the tie is named only at the body step.

### What does the reader say about a neighbour of a question with its body?
depends: be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, 46fc51ff-66b9-4841-a953-ae0d4c2fc424=A|C
id: c6e0b7d6-cdec-46da-b2bc-cda218673868
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, neighbour=daec9ffd-676f-4111-a359-2827bfd4527c, options=43778946-d44c-4acf-a766-0a4915fce199, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, reader=be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, supersede=2bae531b-c4df-48a0-89a0-3d52726741ea

Problem: with the options in view the relation to a decision can be read, and a free sentence cannot be acted on; a word from a closed set can.
Standard: classification into a closed set with an explicit none-of-these, so the reader is never forced.

- A) One of same, consistent, inconsistent, depends, unrelated, after one sentence of reason.
  Why: every relation seen between neighbouring decisions falls into one of the five; unrelated is the exit, no word is forced.
  Costs: a bundled decision gets one word for several rules; the reason sentence names the clause it matched.
- B) One of same, inconsistent, unrelated, after one sentence of reason.
  Why: three words, each with its own move: drop, supersede, nothing.
  Costs: a missing tie is not named; the agent finds depends on its own among the ten.
- C) A free sentence on how the two relate, no word.
  Why: nothing is forced into a set; every nuance of a bundled decision survives.
  Costs: nothing in the record can be compared or counted; the agent reads a sentence per neighbour and rules alone.

**Answer:** A

### Does the reader's word stop a command?
depends: be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, c6e0b7d6-cdec-46da-b2bc-cda218673868=*
id: 902bd279-e6c1-4a21-9637-a3a43f360bae

Problem: a reader is a model; a wrong word that stops a command has no exit but rewording the text, and a word that is only a line leaves a trace.
Standard: advisory checks inform; blocking checks are deterministic.

- A) No.
  Why: the agent's line is the ruling; the reader's word beside it shows the owner where they disagree.
  Costs: a wrong line of the agent passes approve; it is seen, not stopped.
- B) Yes.
  Why: a same from the reader stops the heading until it is reworded or dropped.
  Costs: a wrong same has no exit but rewording; the model's word becomes a rule that cannot be checked again.

### What does the tool keep of the reader's word?
depends: be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, c6e0b7d6-cdec-46da-b2bc-cda218673868=*
id: 08a1b488-32fc-4dc7-b4b3-93a6e768725f

Problem: a word that stays in the terminal is gone at approval; the owner sees the agent's line with nothing against it, and a pair whose texts did not change is read again.
Standard: expensive judgments are stored keyed by their inputs, in a file the tool owns.

- A) A row in its own file under adrdom/state: text hash, neighbour identifier, word, reason, prompt hash, model.
  Why: the neighbour list is the same rows, so approve reads them anywhere; the word and reason meet the agent's line.
  Costs: a second append-only file beside the ledger, hundreds of rows per plan, held like the ledger.
- B) A row in the ledger: the same fields in the ledger's columns.
  Why: one log, one set of rules, nothing new to hold.
  Costs: hundreds of reader rows around tens of owner events, and two keys forced into one identifier column.
- C) Only the word, in its own file, no reason.
  Why: short rows; the reason is read once in the terminal.
  Costs: the owner sees a word against the agent's line and no reason; a bundled decision's clause is not named.

### Does check hold the reader's file as it holds the ledger?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, 08a1b488-32fc-4dc7-b4b3-93a6e768725f=A
id: 26c5352f-3f82-4376-9dc2-8ccc0074d080

### Does planning stop without the reader?
depends: be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, 62e50382-db1b-4649-9f26-7f44c379d6fe=*
id: 3dba486c-a399-4ce3-a794-e38c4f733403

### Does show print the neighbours of a question with the reader's word?
depends: 277a8338-c57a-43cc-8b28-6b42e9347f6a, 08a1b488-32fc-4dc7-b4b3-93a6e768725f=*
id: 97c679f6-e8e6-4970-b547-940477753d7d
