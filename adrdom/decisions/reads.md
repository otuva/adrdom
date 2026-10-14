# Reads

What the agent reads from the tool: what show prints, what the commit hooks
print, what the agent writes on a line the tool turns into identifiers, and
what check tells the agent about a decision whose tier is still open.

### Does every sentence of the brief come from a decision?
depends: 1d9df88a-40f6-4235-a083-e063fde634fe, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, 1b98cdea-dc1b-4789-8f67-68d130d7a49d
id: de5ed5b1-3a83-4403-8953-941cb7e94390
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answered=b36113a1-1384-4e4c-a798-8399f4181366, brief=1d9df88a-40f6-4235-a083-e063fde634fe, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: a sentence written into the brief by hand carries a rule no decision gave, and the agent follows it.
Standard: generated documentation is built from the source of truth and never edited in place.

- A) Yes.
  Why: the brief is as long as the decisions, and each sentence has an identifier behind it.
  Costs: a sentence the brief needs with no decision behind it waits for a question.
- B) No.
  Why: the brief can explain more than the decisions say.
  Costs: a brief sentence binds the agent with nothing the owner answered.

**Answer:** A

### What turns the supersedes line into identifiers?
depends: 2bae531b-c4df-48a0-89a0-3d52726741ea, e877e4a9-2caf-4fe6-9f82-5476a2abb00b, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, 3c8d4c9c-abca-4faa-b7c1-aae39951376c
supersedes: 3b8da790-ea0c-4ca5-860a-980c7bc5f361
unrelated: 7a1e7b81-0967-4ed2-8a8a-c7153056836f, 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f, a82db5b7-969c-44e9-9eec-876ead7099d3, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, c6da863d-5085-402f-815a-9bb832b4df7e, 83190b10-8060-47c1-8194-f6a5c4f0d703, d2851ea1-fcb6-476a-b321-c813a594d5e5, 08a1b488-32fc-4dc7-b4b3-93a6e768725f, f4f667ad-8714-4e65-897b-c7002e423c28, f5c242b6-e2f6-45a3-8c61-c743f3ad794f, 46fc51ff-66b9-4841-a953-ae0d4c2fc424, 178a232e-b51c-4122-a1a9-9e9ed6970748, 277a8338-c57a-43cc-8b28-6b42e9347f6a
id: f19e7914-ad97-4f6b-87dd-9301e53cd068
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, checked=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, identifiers=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, stands on=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, supersedes=2bae531b-c4df-48a0-89a0-3d52726741ea, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: the agent writes a supersedes line with the heading of the decision it replaces; one command turns the heading into its identifier, and which one sets when the line is written.
Standard: one rule for every reference in a topic file.

- A) Approve: the line belongs to the plan, written under the heading before approval, turned like a depends line.
  Why: the owner approves what a question replaces together with what it stands on, and one command turns every reference.
  Costs: a line written after approval waits for a second approval, and the target's currency is checked again at ask.
- B) Ask: the line is written with the body, after approval; ask turns it and freezes it with the body.
  Why: the plan comes first and the body with its line second; nothing under a heading changes between approval and ask.
  Costs: the line is no part of the plan the owner approves; approve turns one kind of reference, ask another.
- C) Whichever command sees a heading on the line.
  Why: the agent writes the line whenever it finds the need.
  Costs: two commands hold the same rule.

**Answer:** A
