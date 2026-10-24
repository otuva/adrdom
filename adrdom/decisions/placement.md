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

**Answer:** A

### What does answer write for a removed question?
depends: e18b82d8-5271-4385-bbab-2b89e5da278f, 462e22c6-8b23-4061-9a2a-bbf8691fce60=A
id: f5c242b6-e2f6-45a3-8c61-c743f3ad794f
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: a question leaves the file at the owner's answer; without a row the approved set and the file disagree and nothing says why.
Standard: an append-only log records every removal with its cause.

- A) A dropped row: the question's identifier, its hash from approval, the identifier and letter of the dropping answer.
  Why: check verifies the drop from the row: that answer was given and the set lacked that letter.
  Costs: one row per removed question, dependents included, each naming the question it stood on.
- B) A dropped row with the question's identifier and its hash from approval, no cause.
  Why: the shortest row that keeps the approved set whole.
  Costs: check sees the removal but cannot verify it was legal; the cause lives in git history only.

**Answer:** A

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

**Answer:** A

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

### What word does the reader give a neighbour of a question with its body?
depends: be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, daec9ffd-676f-4111-a359-2827bfd4527c, 277a8338-c57a-43cc-8b28-6b42e9347f6a, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, 46fc51ff-66b9-4841-a953-ae0d4c2fc424
supersedes: c6e0b7d6-cdec-46da-b2bc-cda218673868, 08a1b488-32fc-4dc7-b4b3-93a6e768725f, 332ff0c7-0769-45ac-8f89-00aeff318467, 62e50382-db1b-4649-9f26-7f44c379d6fe, 902bd279-e6c1-4a21-9637-a3a43f360bae, 908bddc6-5d30-4bcc-be8d-4c470c6bff11, 2db33118-82b4-4edf-a9cb-e92ac62738b9, 4b407ee1-1e64-4ca1-842b-69773fc0e48a, 36dfbeaa-698d-4a57-b3b0-d425348e14a5, 848e1b76-9cfe-483b-94dd-094eb9b2f995, 3fc35d4f-90f3-4425-9dac-951a89631fdf, bceceda8-133a-4c9b-b439-b5c1a4414a02, c84a4bd9-fc2d-44f0-b333-172978f61544, 8f81dee1-9c19-447d-baa8-d7d0af691db8, a6f41dee-2381-421a-bcc4-b14af243209b, 83190b10-8060-47c1-8194-f6a5c4f0d703, 3dba486c-a399-4ce3-a794-e38c4f733403, 4838baa4-a5df-49e7-929c-ed63b5adaf4e, 1ab6d6c3-7e44-42de-bc2f-2e5849114aef, 90714223-52e3-4bb9-b0b4-2a8b87d74145, abb6faed-96f7-406d-8384-002ee81bb157, bc4291c8-98f1-499d-8e02-46e371035e70
unrelated: 11be073c-ce91-44d8-833a-ef0b5e0b1672, c6451ecc-17a9-447e-a9c9-fe9b35ec789c, 55a8abb2-0f6d-430b-99f9-a378f3f154e5, 9d634996-475e-4ddc-8113-05c404ad9d5c, 6108b8b2-911e-443b-abe0-4c6f8b407384, c3127af2-4c96-48d5-a164-bac49a9a09ec, b782664f-7465-4e1e-99e0-13f6dd2de774, 28b5837d-46ae-459e-ba4c-12265ef07ac1, e78b8297-fd89-4e09-9c3e-9eeb0fd0f654, bd94a89f-7142-4dab-b757-6482a718a0a0, 60341be0-fd05-4281-92d7-668584a8539a, a8034401-420c-45d7-8f43-60c1b9f3baf9, d984e425-3e88-4756-88fb-18241f9f62a7, 30ca5d13-e68b-465f-aa84-ea4000c9330a, 55a6a2d8-03af-4db9-a4c7-e2790e38357e, b36113a1-1384-4e4c-a798-8399f4181366
id: d10dab89-1d42-4764-9d72-22d6fd6a9260
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, neighbour=daec9ffd-676f-4111-a359-2827bfd4527c, option=43778946-d44c-4acf-a766-0a4915fce199, options=43778946-d44c-4acf-a766-0a4915fce199, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, reader=be0d6f5c-ae1b-4753-8ec7-0f4b03d84641

Problem: with the options in view the relation to a decision can be read, and a free sentence cannot be acted on; a word from a closed set can.
Standard: classification into a closed set with an explicit none-of-these, so the reader is never forced.

- A) One of same, consistent, inconsistent, unrelated, or closes with the letter it leaves standing, after one sentence of reason.
  Why: each word is read off the two texts; closes names the option the record forces, and unrelated is the exit.
  Costs: a bundled decision gets one word for several rules; the reason sentence names the clause it matched.
- B) One of same, consistent, inconsistent, depends, unrelated, after one sentence of reason.
  Why: a missing tie is named by the reader beside the agent's line.
  Costs: depends is a claim about order two texts do not show, and the word gets copied into the lines.
- C) A free sentence on how the two relate, no word.
  Why: nothing is forced into a set; every nuance of a bundled decision survives.
  Costs: nothing in the record can be compared or counted; the agent reads a sentence per neighbour and rules alone.

**Answer:** A

### Does the reader read a neighbour of a heading?
depends: be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, daec9ffd-676f-4111-a359-2827bfd4527c, d10dab89-1d42-4764-9d72-22d6fd6a9260=A|B
supersedes: 62e50382-db1b-4649-9f26-7f44c379d6fe, 3dba486c-a399-4ce3-a794-e38c4f733403, 4838baa4-a5df-49e7-929c-ed63b5adaf4e
unrelated: c6451ecc-17a9-447e-a9c9-fe9b35ec789c, 902bd279-e6c1-4a21-9637-a3a43f360bae, 55a8abb2-0f6d-430b-99f9-a378f3f154e5, 332ff0c7-0769-45ac-8f89-00aeff318467, c6e0b7d6-cdec-46da-b2bc-cda218673868, de8ef568-3a8d-42ba-bef7-d7fd467b27be, 43778946-d44c-4acf-a766-0a4915fce199, 277a8338-c57a-43cc-8b28-6b42e9347f6a, 7a1e7b81-0967-4ed2-8a8a-c7153056836f, 6108b8b2-911e-443b-abe0-4c6f8b407384, c3127af2-4c96-48d5-a164-bac49a9a09ec, 46fc51ff-66b9-4841-a953-ae0d4c2fc424, e78b8297-fd89-4e09-9c3e-9eeb0fd0f654, bd94a89f-7142-4dab-b757-6482a718a0a0, a8034401-420c-45d7-8f43-60c1b9f3baf9, d984e425-3e88-4756-88fb-18241f9f62a7, 30ca5d13-e68b-465f-aa84-ea4000c9330a, 55a6a2d8-03af-4db9-a4c7-e2790e38357e
id: 60341be0-fd05-4281-92d7-668584a8539a
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, neighbour=daec9ffd-676f-4111-a359-2827bfd4527c, neighbours=daec9ffd-676f-4111-a359-2827bfd4527c, options=43778946-d44c-4acf-a766-0a4915fce199, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, reader=be0d6f5c-ae1b-4753-8ec7-0f4b03d84641

Problem: at the heading step the new side has no options and no answer; a word given there is copied into the lines the agent writes.
Standard: a classifier gets only the labels its input can support.

- A) No.
  Why: the agent reads the ten neighbours itself and writes its lines; the reader reads the question with its body.
  Costs: a heading a decision settles gets a body before it drops.
- B) Yes.
  Why: a heading a decision settles drops before its body is written.
  Costs: ten reader calls per heading, repeated at the body step, and a word on hierarchy the text cannot support.

**Answer:** A

### What does the tool keep of a reader's word?
depends: be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, daec9ffd-676f-4111-a359-2827bfd4527c, d10dab89-1d42-4764-9d72-22d6fd6a9260=*
supersedes: 08a1b488-32fc-4dc7-b4b3-93a6e768725f, 1ab6d6c3-7e44-42de-bc2f-2e5849114aef, 2db33118-82b4-4edf-a9cb-e92ac62738b9, 332ff0c7-0769-45ac-8f89-00aeff318467, 90714223-52e3-4bb9-b0b4-2a8b87d74145, abb6faed-96f7-406d-8384-002ee81bb157, 36dfbeaa-698d-4a57-b3b0-d425348e14a5, 848e1b76-9cfe-483b-94dd-094eb9b2f995, 3fc35d4f-90f3-4425-9dac-951a89631fdf, bceceda8-133a-4c9b-b439-b5c1a4414a02, c84a4bd9-fc2d-44f0-b333-172978f61544, 8f81dee1-9c19-447d-baa8-d7d0af691db8, a6f41dee-2381-421a-bcc4-b14af243209b, 83190b10-8060-47c1-8194-f6a5c4f0d703, 4b407ee1-1e64-4ca1-842b-69773fc0e48a, 908bddc6-5d30-4bcc-be8d-4c470c6bff11, bc4291c8-98f1-499d-8e02-46e371035e70
unrelated: 902bd279-e6c1-4a21-9637-a3a43f360bae, ec0b9293-916e-4ced-a3d7-3b862ab7b069, 149c9a73-16eb-46aa-9806-2950423ee1a2, bf7da2dd-d8a7-41b8-9898-11cf68d0e833, 209081f4-864f-4ef6-8909-65da87f135e5, 7a1e7b81-0967-4ed2-8a8a-c7153056836f, c6da863d-5085-402f-815a-9bb832b4df7e, 6108b8b2-911e-443b-abe0-4c6f8b407384, ef7ec04d-9cdd-40a1-8246-42a4a0200828, 16a71892-b348-4b9b-82c5-a9d375187bd3, f422c5db-13d0-4ff7-9e43-b68ab44a7cb9, aa475aa3-05b2-429a-b33a-46b1dcb52049, e78b8297-fd89-4e09-9c3e-9eeb0fd0f654, bd94a89f-7142-4dab-b757-6482a718a0a0, 60341be0-fd05-4281-92d7-668584a8539a, a8034401-420c-45d7-8f43-60c1b9f3baf9, d984e425-3e88-4756-88fb-18241f9f62a7, 30ca5d13-e68b-465f-aa84-ea4000c9330a
id: 55a6a2d8-03af-4db9-a4c7-e2790e38357e
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, neighbour=daec9ffd-676f-4111-a359-2827bfd4527c, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, reader=be0d6f5c-ae1b-4753-8ec7-0f4b03d84641

Problem: a word that stays in the terminal is gone at approval; the owner sees the agent's line with nothing against it, and a pair whose texts did not change is read again.
Standard: expensive judgments are stored keyed by their inputs, in a file the tool owns.

- A) A row in its own file under adrdom/state: text hash, neighbour identifier, word, reason, prompt hash, model.
  Why: the neighbour list is the same rows, so approve reads them anywhere; word and reason meet the agent's line.
  Costs: a second append-only file beside the ledger, hundreds of rows per plan, held like the ledger.
- B) A row in the ledger: the same fields in the ledger's columns.
  Why: one log, one set of rules, nothing new to hold.
  Costs: hundreds of reader rows around tens of owner events, and two keys forced into one identifier column.
- C) Only the word, in its own file, no reason.
  Why: short rows; the reason is read once in the terminal.
  Costs: the owner sees a word against the agent's line, no reason; a bundled decision's clause is not named.

**Answer:** A

### Does a reader's word stop a command?
depends: be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, f90dbd5b-8fbe-4899-8588-f8762795c2e2, d10dab89-1d42-4764-9d72-22d6fd6a9260=*
supersedes: 902bd279-e6c1-4a21-9637-a3a43f360bae
unrelated: f53ac3b9-efde-427c-8027-19988aa8fda7, 80b89c24-7d65-43f4-bb03-55e9d85c327d, 8081c82d-e5c6-485b-a302-07d586f1a01b, 7cf81ded-be48-4777-b83e-f8a2998c1ab5, ef7ec04d-9cdd-40a1-8246-42a4a0200828, aa475aa3-05b2-429a-b33a-46b1dcb52049, 3cf56867-3918-4ce6-a296-2cc62a91b154, 55a8abb2-0f6d-430b-99f9-a378f3f154e5, 8846f6d1-cd8c-4b87-b6f2-35bc8adf6763, 735a32bd-db0c-428e-b65e-2acc750671b0, b0e865b4-9204-4c55-9f5b-dcb755b3a03b, 4f04bae4-98e3-48eb-9403-3257268e2d87, 6108b8b2-911e-443b-abe0-4c6f8b407384, c3127af2-4c96-48d5-a164-bac49a9a09ec, 98d5d389-8315-41da-9970-666e31f3a94d, bd94a89f-7142-4dab-b757-6482a718a0a0, 60341be0-fd05-4281-92d7-668584a8539a, a8034401-420c-45d7-8f43-60c1b9f3baf9, d984e425-3e88-4756-88fb-18241f9f62a7, 30ca5d13-e68b-465f-aa84-ea4000c9330a, 55a6a2d8-03af-4db9-a4c7-e2790e38357e
id: e78b8297-fd89-4e09-9c3e-9eeb0fd0f654
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, checked=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, checks=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, reader=be0d6f5c-ae1b-4753-8ec7-0f4b03d84641

Problem: a reader is a model; a wrong word that stops a command has no exit but rewording the text, and a word that is only a line leaves a trace.
Standard: advisory checks inform; blocking checks are deterministic.

- A) No.
  Why: the agent's line is the ruling; the reader's word beside it shows the owner where they disagree.
  Costs: a wrong line of the agent passes approve; it is seen, not stopped.
- B) Yes.
  Why: a same from the reader stops the heading until it is reworded or dropped.
  Costs: a wrong same has no exit but rewording; the model's word becomes a rule that cannot be checked again.

**Answer:** A

### Does planning stop with no reader on the machine?
depends: be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, 94677ed8-eaf6-46e2-bc71-7392fa0805bf, 60341be0-fd05-4281-92d7-668584a8539a=*
supersedes: 3dba486c-a399-4ce3-a794-e38c4f733403
unrelated: d3be71e4-ce7b-43b0-86a8-e86868521f99, 21cdad59-4f0c-42a5-b4f3-6af946637df3, c3127af2-4c96-48d5-a164-bac49a9a09ec, 1cec44fc-2685-42f8-9742-7facabb52251, 90714223-52e3-4bb9-b0b4-2a8b87d74145, 902bd279-e6c1-4a21-9637-a3a43f360bae, 6c406624-b874-4697-9a44-91941019b7d1, 49a1c3a8-07ea-448a-8926-2362c0617758, 3aedf314-f25f-49cb-b7b3-d2302e4543ca, 3fc35d4f-90f3-4425-9dac-951a89631fdf, 848e1b76-9cfe-483b-94dd-094eb9b2f995, 08a1b488-32fc-4dc7-b4b3-93a6e768725f, 8f81dee1-9c19-447d-baa8-d7d0af691db8, e78b8297-fd89-4e09-9c3e-9eeb0fd0f654, a8034401-420c-45d7-8f43-60c1b9f3baf9, d984e425-3e88-4756-88fb-18241f9f62a7, 30ca5d13-e68b-465f-aa84-ea4000c9330a, 55a6a2d8-03af-4db9-a4c7-e2790e38357e, d10dab89-1d42-4764-9d72-22d6fd6a9260
id: bd94a89f-7142-4dab-b757-6482a718a0a0
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, neighbour=daec9ffd-676f-4111-a359-2827bfd4527c, neighbours=daec9ffd-676f-4111-a359-2827bfd4527c, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, reader=be0d6f5c-ae1b-4753-8ec7-0f4b03d84641

Problem: the reader is a program apart from the tool; a machine without it brings neighbours with no word, and a plan with wordless rows is a plan with a hole.
Standard: a step whose input is missing stops and names it; no degraded mode.

- A) Yes.
  Why: one shape of row; every neighbour read with a body carries a word.
  Costs: planning needs the reader on the machine; every other command runs without it.
- B) No.
  Why: the agent reads the neighbours itself; the word is a line, not a gate, so its absence stops nothing.
  Costs: two shapes of row, and the owner sees agent lines with nothing against them.

**Answer:** A

### Does the tool bring a plan's own questions as neighbours of each other?
depends: 94677ed8-eaf6-46e2-bc71-7392fa0805bf, daec9ffd-676f-4111-a359-2827bfd4527c, 60341be0-fd05-4281-92d7-668584a8539a=*
supersedes: 4838baa4-a5df-49e7-929c-ed63b5adaf4e
unrelated: 9ba58b6a-bc85-44a5-a720-d8e7a6ff7b31, 98d5d389-8315-41da-9970-666e31f3a94d, 4b407ee1-1e64-4ca1-842b-69773fc0e48a, 1ab6d6c3-7e44-42de-bc2f-2e5849114aef, 010901e0-c02d-49eb-98bc-d584756d7f51, 0e38f001-0226-4927-9a78-2e9fc1ec18d4, de8ef568-3a8d-42ba-bef7-d7fd467b27be, 6c406624-b874-4697-9a44-91941019b7d1, 3aedf314-f25f-49cb-b7b3-d2302e4543ca, 7a1e7b81-0967-4ed2-8a8a-c7153056836f, c6451ecc-17a9-447e-a9c9-fe9b35ec789c, 23f21671-e34c-44aa-8804-6163d975807f, 62e50382-db1b-4649-9f26-7f44c379d6fe, c6e0b7d6-cdec-46da-b2bc-cda218673868, e78b8297-fd89-4e09-9c3e-9eeb0fd0f654, bd94a89f-7142-4dab-b757-6482a718a0a0, a8034401-420c-45d7-8f43-60c1b9f3baf9, 30ca5d13-e68b-465f-aa84-ea4000c9330a, 55a6a2d8-03af-4db9-a4c7-e2790e38357e, d10dab89-1d42-4764-9d72-22d6fd6a9260, 277a8338-c57a-43cc-8b28-6b42e9347f6a
id: d984e425-3e88-4756-88fb-18241f9f62a7
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, neighbour=daec9ffd-676f-4111-a359-2827bfd4527c, neighbours=daec9ffd-676f-4111-a359-2827bfd4527c, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, reader=be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c

Problem: two new headings in one plan are never each other's neighbour when the pool is the records; the tie between them is written by hand and forgotten.
Standard: deduplicate a batch against itself as well as against the store.

- A) Yes.
  Why: at the body step each question is read against the plan's nearest questions with the heading set.
  Costs: a few more reader calls per question; two unanswered questions give no consistent or inconsistent.
- B) No.
  Why: the pool is the records; the agent wrote both headings and knows their tie.
  Costs: two headings on one rule in one plan pass; the tier cap is the only net.

**Answer:** A

### Does the reader's file keep a row whose text left the records?
depends: be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, daec9ffd-676f-4111-a359-2827bfd4527c, 55a6a2d8-03af-4db9-a4c7-e2790e38357e=*
supersedes: 90714223-52e3-4bb9-b0b4-2a8b87d74145, abb6faed-96f7-406d-8384-002ee81bb157
unrelated: b0554224-15e4-4e91-a2bb-9ae5667803f4, bf6697e3-8832-4b90-b036-8fecedb55406, c10c3a49-e993-4c9f-a46b-4a1ba3646dcf, bceceda8-133a-4c9b-b439-b5c1a4414a02, 385a57d5-5f76-454a-ae5d-9b0d6f5c1492, 55a8abb2-0f6d-430b-99f9-a378f3f154e5, 08a1b488-32fc-4dc7-b4b3-93a6e768725f, 1c27f2d6-ab62-4c5a-91b2-1488722f3c48, cd83e277-ebf0-4458-a77a-c526ab4cfeca, 99619e27-19b6-47ef-83b9-c79ac30ee2f0, f3e64c4e-21ab-442f-8c40-6060591e2b77, e78b8297-fd89-4e09-9c3e-9eeb0fd0f654, bd94a89f-7142-4dab-b757-6482a718a0a0, 60341be0-fd05-4281-92d7-668584a8539a, d984e425-3e88-4756-88fb-18241f9f62a7, 30ca5d13-e68b-465f-aa84-ea4000c9330a, d10dab89-1d42-4764-9d72-22d6fd6a9260
id: a8034401-420c-45d7-8f43-60c1b9f3baf9
terms: ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, neighbour=daec9ffd-676f-4111-a359-2827bfd4527c, reader=be0d6f5c-ae1b-4753-8ec7-0f4b03d84641

Problem: the file grows by a row for every reading of a text since edited away, and a row nothing can look up again is weight on every read.
Standard: a cache keyed by content hash is pruned of entries whose key no longer resolves; the record of events is kept elsewhere.

- A) No.
  Why: when it appends, the tool drops each row whose text or neighbour is gone from the records; git keeps them.
  Costs: a merge brings a dropped row back from the other branch until the next append drops it again.
- B) Yes.
  Why: every reading ever made stays; the file is a record like the ledger.
  Costs: a row per edited draft text for ever, read on every command.

**Answer:** A

### What does check hold of a reader's file?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, a8034401-420c-45d7-8f43-60c1b9f3baf9=A|B, 55a6a2d8-03af-4db9-a4c7-e2790e38357e=*
supersedes: abb6faed-96f7-406d-8384-002ee81bb157
unrelated: 34e81f57-9b3c-4dc4-a2ba-ca227523177d, 6ef9eca6-fd8b-4211-aae7-00b37a2a9245, ec3f875c-a0f8-4085-8da1-fec9dcb14377, 6108b8b2-911e-443b-abe0-4c6f8b407384, bceceda8-133a-4c9b-b439-b5c1a4414a02, 90714223-52e3-4bb9-b0b4-2a8b87d74145, 05141f49-d9d2-407c-b60c-ebb195a05864, a6f41dee-2381-421a-bcc4-b14af243209b, 08a1b488-32fc-4dc7-b4b3-93a6e768725f, 7a1e7b81-0967-4ed2-8a8a-c7153056836f, bf6697e3-8832-4b90-b036-8fecedb55406, c10c3a49-e993-4c9f-a46b-4a1ba3646dcf, d2851ea1-fcb6-476a-b321-c813a594d5e5, f3e64c4e-21ab-442f-8c40-6060591e2b77, e78b8297-fd89-4e09-9c3e-9eeb0fd0f654, bd94a89f-7142-4dab-b757-6482a718a0a0, 60341be0-fd05-4281-92d7-668584a8539a, d984e425-3e88-4756-88fb-18241f9f62a7, d10dab89-1d42-4764-9d72-22d6fd6a9260
id: 30ca5d13-e68b-465f-aa84-ea4000c9330a
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, reader=be0d6f5c-ae1b-4753-8ec7-0f4b03d84641

Problem: a file the tool writes and the agent may edit is a file the agent will edit; a rule that holds every row at HEAD byte for byte holds a row the tool itself drops.
Standard: a machine-written cache is verified for shape and against edits; removal is the writer's act.

- A) Every row at HEAD stays byte for byte or is gone whole; no row is edited or added by hand.
  Why: a hand-edited row never passes as the reader's word; the tool's own drop passes.
  Costs: a row deleted by hand passes as a drop; a hand-written row is caught by its shape alone.
- B) Every row at HEAD stays byte for byte.
  Why: the file is held like the ledger.
  Costs: the tool cannot drop a row, so the file keeps every reading ever made.
- C) The shape of each row alone.
  Why: nothing to compare with history.
  Costs: a row edited by hand passes as the reader's word.

**Answer:** A

### Does the agent write an unrelated line?
depends: agent, neighbour, depends, supersedes, What does the agent do to plan a topic file?
supersedes: Does every neighbour get a line under its heading?
unrelated: Does a reference line keep its list on one line?, Does check print a line with no move on it?, Does install write outside adrdom?, Does the agent build what no current decision covers?, Does the agent that asks a question build its decision?, Does the agent write the bodies before approve?, Does the agent write the plan?, Does the tool rewrite the ban list from the ledger?, Does the tool write a terms brief for the agent?, What does install write for one agent?, Does a supersedes line name every current decision standing on its target?, Does the reader read a neighbour of a heading?, What does show print of a reader's word on a question?, What stops a question that has no depends line?, What word does the reader give a neighbour of a question with its body?, Does check read the words of an answered question again?, Does the body step read the neighbours brought for the heading?

Problem: a line for every neighbour grows to thirty names under a question, no move reads them, and each new neighbour costs a run.
Standard: a record holds the rulings, not the list of what was looked at.

- A) No.
  Why: depends and supersedes are the rulings; a neighbour on neither is the reader's to dispute at approval.
  Costs: the line stays in answered questions, read and judged by nothing; a neighbour never looked at leaves no trace.
- B) Yes.
  Why: every neighbour the tool brings meets a written ruling, and a forgotten one is a finding.
  Costs: thirty names per question, a run for each new neighbour, and a word no move reads.

### Does the body step read the neighbours brought for the heading?
depends: reader, neighbour, What word does the reader give a neighbour of a question with its body?
supersedes: Does the reader read a neighbour of a heading?, Does planning stop with no reader on the machine?, Does the tool bring a plan's own questions as neighbours of each other?
unrelated: Does every neighbour get a line under its heading?, question, Does the tool read each current decision against its neighbours?, Does the agent write the bodies before approve?, What does show print of a reader's word on a question?, What does the tool bring as neighbours for a new heading?, Does the word limit read the text of an answered question?, Does ask pair the whole question to terms?, Does check read the words of an answered question again?, What does the agent do to plan a topic file?, Does the agent write an unrelated line?, Does a planning step run without the record of the step before?, Does the tool print each pair the reader marks?, move

Problem: the neighbours brought for a heading get no word from the reader, so a decision among them that the body meets is seen by the agent alone.
Standard: a reviewer reads every item on the list with the full text in hand.

- A) Yes.
  Why: the same prompt and word set, options in view; a heading's neighbour the body meets gets its word.
  Costs: up to ten more pairs per question, each a reader call, most of them unrelated.
- B) No.
  Why: the reader reads the ten nearest of the body and the own questions; the heading has the agent.
  Costs: a decision near the heading and far from the body gets no word.

### Does planning stop when the reader is missing?
depends: reader, plan, Does the body step read the neighbours brought for the heading?=*
unrelated: Does a planning step run without the record of the step before?, Does a reader's word stop a command?, Does ask stop a question outside the plan?, Does check fail a planner's commit that reaches outside adrdom?, Does planning stop with no reader on the machine?, Does planning stop without the embedding model?, Does the ledger hold the planners?, Does the reader read a neighbour of a heading?, What makes a session a planner?, planner, Does the agent build what no current decision covers?, Does the reader's file keep a row whose text left the records?, What does check hold of a reader's file?, What does the tool keep of a reader's word?, What does the tool print for a build move in a planner session?, Does the agent write an unrelated line?, Does the body step bring the plan's own questions as neighbours?

Problem: the reader is a program apart from the tool; a machine without it brings neighbours with no word, and a plan with wordless rows is a plan with a hole.
Standard: a step whose input is missing stops and names it; no degraded mode.

- A) Yes.
  Why: one shape of row; every neighbour read with a body carries a word.
  Costs: planning needs the reader on the machine; every other command runs without it.
- B) No.
  Why: the agent reads the neighbours itself; the word is a line, not a gate, so its absence stops nothing.
  Costs: two shapes of row, and the owner sees agent lines with nothing against them.

### Does the body step bring the plan's own questions as neighbours?
depends: plan, neighbour, Does the body step read the neighbours brought for the heading?=*
unrelated: Does a command compute a move of its own?, Does a planning step run without the record of the step before?, Does every neighbour get a line under its heading?, Does the agent that asks a question build its decision?, Does the agent write the bodies before approve?, Does the reader read a neighbour of a heading?, Does the tool bring a plan's own questions as neighbours of each other?, Does the tool read each current decision against its neighbours?, What word does the reader give a neighbour of a question with its body?, move, Does a question keep its identifier when its heading is reworded?, Does ask pair the whole question to terms?, Does one question heading stand in two topic files?, Does the agent build on a decision before its tier is fully answered?, What does the agent do to plan a topic file?, Does planning stop when the reader is missing?, Does the agent write an unrelated line?

Problem: two new headings in one plan are never each other's neighbour when the pool is the records; the line between them is written by hand and forgotten.
Standard: deduplicate a batch against itself as well as against the store.

- A) Yes.
  Why: at the body step each question is read against the plan's nearest questions with the heading set.
  Costs: a few more reader calls per question; two unanswered questions give no consistent or inconsistent.
- B) No.
  Why: the pool is the records; the agent wrote both headings and knows their line.
  Costs: two headings on one rule in one plan pass; the tier cap is the only net.
