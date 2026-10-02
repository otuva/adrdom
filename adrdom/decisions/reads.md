# Reads

What the agent reads from the tool: what show prints, what the commit hooks
print, what the agent writes on a line the tool turns into identifiers, and
what check tells the agent about a decision whose tier is still open.

### Does show print every question?
depends: 277a8338-c57a-43cc-8b28-6b42e9347f6a, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, 08193e75-dd89-4857-8e9f-b19e1a9e75e6, 7b05f8df-09a5-404e-a0f0-3dd8824d1d15
id: 75075c78-fe3c-4fd8-8a33-29a5913d9d34
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, blame file=108053a8-c2c3-4561-874c-2f573a5409f2, blame files=108053a8-c2c3-4561-874c-2f573a5409f2, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, option=43778946-d44c-4acf-a766-0a4915fce199, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, refused=0942e445-0163-423e-8ba7-84dd8ac07045, stands on=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, superseded=2bae531b-c4df-48a0-89a0-3d52726741ea, tiers=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6, withdrawn=e4a2ef0c-3354-4c9b-a8d9-9e204e544d85

Problem: an agent that runs show to find its next move reads every question of every topic file, and one that writes a question into a file needs that file's ties and nothing else.
Standard: a listing command prints a summary and takes a flag or an argument for the whole.

- A) No: show prints every current decision as its heading and the option picked, the next lines of every topic file and of every blame file, and one line naming the two fuller forms; show with a topic file prints every question of that file with what it stands on, what stands on it and what took the place of a replaced one; show with the flag for the whole prints that for every file and the blame files.
  Why: the agent reads the move first, the decisions second, and one file's ties when it writes into that file.
  Costs: refused, superseded and withdrawn questions and the blame files are read through the flag or the file form; the decisions that say show prints the tiers and the blame files hold for the fuller forms alone; the next lines of a blame file join the short listing.
- B) Yes: show prints every question with its state and ties, the tiers, the next lines and the blame files, in one listing.
  Why: one listing holds everything, nothing hides behind a flag.
  Costs: the agent reads hundreds of lines to find one move, and every question written adds to what every later run prints.
- C) No: show prints the next lines alone; the file form and the flag as in A.
  Why: the shortest listing.
  Costs: the current decisions, which the agent reads before it builds, need a second command.

**Answer:** A

### Does a hook print what check prints on a pass?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, 1b98cdea-dc1b-4789-8f67-68d130d7a49d, 34e81f57-9b3c-4dc4-a2ba-ca227523177d
id: 154e4b9a-c811-4a84-a522-598914eb39f1
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: the two commit hooks each run check, and on a pass each prints its notes, a tier line and a done line per topic file and a tested count, so every commit puts twice that on the agent's screen with nothing to act on.
Standard: a hook is silent on success and prints only what failed.

- A) No: a hook runs check with a flag under which a pass prints nothing and a failure prints the failing lines alone; check by hand prints its notes as before without the flag.
  Why: a commit that passes leaves nothing to read, and a failing one shows only the lines with a move in them.
  Costs: check grows a flag, and the hook files install writes name it.
- B) Yes: a hook prints everything check prints.
  Why: the agent sees at every commit that check ran and what it covered.
  Costs: dozens of lines per commit with no move in them, twice.
- C) No: check prints nothing on a pass, by hand or in a hook.
  Why: one behaviour, no flag.
  Costs: the done and awaiting-approval notes, which the decisions on a topic file closing and on a plan awaiting approval ask check to print, go.

**Answer:** A

### Does approve turn the supersedes line into identifiers?
depends: e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, 3c8d4c9c-abca-4faa-b7c1-aae39951376c, 2bae531b-c4df-48a0-89a0-3d52726741ea, 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f, f4f667ad-8714-4e65-897b-c7002e423c28
id: 3b8da790-ea0c-4ca5-860a-980c7bc5f361
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, asks=3c8d4c9c-abca-4faa-b7c1-aae39951376c, checked=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, identifiers=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, refused=0942e445-0163-423e-8ba7-84dd8ac07045, stands on=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, supersedes=2bae531b-c4df-48a0-89a0-3d52726741ea, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: the agent writes a supersedes line with the heading of the decision it replaces; one command has to turn the heading into its identifier, and the decision on references names approve for the depends line while a supersedes line is written with the body, after approval.
Standard: one rule for every reference in a topic file.

- A) No: ask turns it; the supersedes line is written with the body, after approval, and ask turns each heading into its identifier and freezes the line with the body it asks.
  Why: the agent writes the plan first and the body with its supersedes line second, as the brief orders, and nothing under a heading changes between approval and ask.
  Costs: the supersedes line is no part of the plan the owner approves; a heading that carries the line before approval is refused until its body is written; approve turns one kind of reference and ask another.
- B) Yes: approve turns it; the supersedes line belongs to the plan, written under the heading before approval, and approve turns it like a depends line.
  Why: the owner approves what a question replaces together with what it stands on, and one command turns every reference.
  Costs: a supersedes line written after approval waits for a second approval; the line is turned before the body exists, so the target's currency is checked once more at ask.
- C) Both: whichever command sees a heading on the line turns it.
  Why: the agent writes the line whenever it finds the need.
  Costs: two commands hold the same rule.

**Answer:** B

### Does check fail an untested decision whose tier is still open?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, 33a45b9e-8150-488e-be70-7ab2ca9adaa6, f8d92841-aef7-4cd7-bd94-0e07c03e4cb6, dfd32ef6-c2ac-40ef-b8b3-9c58732516b1
id: f2e31cbe-1ba2-4b77-910a-b9664ce0277f
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, answered=b36113a1-1384-4e4c-a798-8399f4181366, answers=b36113a1-1384-4e4c-a798-8399f4181366, asks=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, refused=0942e445-0163-423e-8ba7-84dd8ac07045, superseded=2bae531b-c4df-48a0-89a0-3d52726741ea, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c

Problem: an owner command commits nothing, so the answers of a tier reach git in the next commit; check at that commit fails every current decision with no test, and the agent must not build a decision while a sibling in its tier is open; after the first answer of a tier of two, the answer cannot be committed without a test the agent must not write.
Standard: a gate fires when its precondition is met, not before; a check that cannot be satisfied yet is reported as waiting, not as failing.

- A) No: check fails an untested decision once every question of its tier is answered, and until then reports it as waiting on the tier, in one note line.
  Why: the answers of a tier are committed one by one and the tier is built whole, as the build decision asks.
  Costs: check reads the tier state to judge a test; the decision that check fails a current decision with no test reads wider than this one until it is superseded.
- B) Yes: check fails it, and the owner answers a tier in one sitting so that the agent builds and commits it whole.
  Why: nothing is added to check; every current decision has a test at every commit.
  Costs: the owner cannot answer half a tier and stop; an answer given alone is refused at commit until its siblings are answered and all are built.
- C) No: check fails it once the tier is answered; until then it says nothing about it.
  Why: nothing to read while waiting.
  Costs: an answered decision with no test and no line naming it.

**Answer:** A

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
