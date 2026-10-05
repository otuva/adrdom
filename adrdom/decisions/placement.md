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

### Does a depends entry on a question in its file carry an answer set?
depends: 9d634996-475e-4ddc-8113-05c404ad9d5c, aef1b384-2b33-47a2-92a8-ec7f8fb258ef, f5eeaacf-1c9f-478b-8a97-2df1aaf39458=*
id: 946263aa-519a-47aa-b743-5cdcf52f5d35

### Does check fail a topic file awaiting approval?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, f5eeaacf-1c9f-478b-8a97-2df1aaf39458=*
id: 912f4f3b-1213-435a-af0d-7617017fcff4

### Does answer remove a question whose answer set lacks the answer?
depends: b36113a1-1384-4e4c-a798-8399f4181366, 946263aa-519a-47aa-b743-5cdcf52f5d35, 61e73795-63de-4669-b09f-48be96919325=A
supersedes: fa90692a-150e-4a89-91e2-1d575cb59d0f
id: 462e22c6-8b23-4061-9a2a-bbf8691fce60

### What does answer write for a removed question?
depends: e18b82d8-5271-4385-bbab-2b89e5da278f, 462e22c6-8b23-4061-9a2a-bbf8691fce60
id: f5c242b6-e2f6-45a3-8c61-c743f3ad794f

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

### What does the agent do to plan a topic file?
depends: f90dbd5b-8fbe-4899-8588-f8762795c2e2, 94677ed8-eaf6-46e2-bc71-7392fa0805bf, de8ef568-3a8d-42ba-bef7-d7fd467b27be=*
id: 46fc51ff-66b9-4841-a953-ae0d4c2fc424

### Does a planning step run without the record of the step before?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, 46fc51ff-66b9-4841-a953-ae0d4c2fc424
id: d3be71e4-ce7b-43b0-86a8-e86868521f99

### Does planning stop without the embedding model?
depends: a7b0b910-57f6-4c59-b904-b42f78db63f7, de8ef568-3a8d-42ba-bef7-d7fd467b27be=*
id: 21cdad59-4f0c-42a5-b4f3-6af946637df3

### Does the tool bring the plan's own questions as neighbours of each other?
depends: 94677ed8-eaf6-46e2-bc71-7392fa0805bf, 62e50382-db1b-4649-9f26-7f44c379d6fe
id: 4838baa4-a5df-49e7-929c-ed63b5adaf4e

### Does every neighbour get a line under its heading?
depends: aef1b384-2b33-47a2-92a8-ec7f8fb258ef, 2bae531b-c4df-48a0-89a0-3d52726741ea, 46fc51ff-66b9-4841-a953-ae0d4c2fc424
id: c6451ecc-17a9-447e-a9c9-fe9b35ec789c

### What does the reader say about a neighbour of a heading?
depends: be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, c6e0b7d6-cdec-46da-b2bc-cda218673868
id: 62e50382-db1b-4649-9f26-7f44c379d6fe

### What does the reader say about a neighbour of a question with its body?
depends: be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, 46fc51ff-66b9-4841-a953-ae0d4c2fc424
id: c6e0b7d6-cdec-46da-b2bc-cda218673868

### Does the reader's word stop a command?
depends: be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, c6e0b7d6-cdec-46da-b2bc-cda218673868
id: 902bd279-e6c1-4a21-9637-a3a43f360bae

### What does the tool keep of the reader's word?
depends: be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, c6e0b7d6-cdec-46da-b2bc-cda218673868
id: 08a1b488-32fc-4dc7-b4b3-93a6e768725f

### Does check hold the reader's file as it holds the ledger?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, 08a1b488-32fc-4dc7-b4b3-93a6e768725f
id: 26c5352f-3f82-4376-9dc2-8ccc0074d080

### Does planning stop without the reader?
depends: be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, 62e50382-db1b-4649-9f26-7f44c379d6fe
id: 3dba486c-a399-4ce3-a794-e38c4f733403

### Does show print the neighbours of a question with the reader's word?
depends: 277a8338-c57a-43cc-8b28-6b42e9347f6a, 08a1b488-32fc-4dc7-b4b3-93a6e768725f
id: 97c679f6-e8e6-4970-b547-940477753d7d
