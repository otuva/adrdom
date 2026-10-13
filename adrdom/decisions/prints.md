# Prints

What a command prints to the one who runs it, in the terminal and on the ui:
the status of what it reads, the findings of the rules it judges, the move
that follows, or the event it wrote; and that status, rule and move are
each computed once.

### Does a command name a status of its own?
depends: 209081f4-864f-4ef6-8909-65da87f135e5, e18b82d8-5271-4385-bbab-2b89e5da278f, 08193e75-dd89-4857-8e9f-b19e1a9e75e6, 277a8338-c57a-43cc-8b28-6b42e9347f6a, f590059d-b681-40ec-8a1c-c8916d71f6bb, 108053a8-c2c3-4561-874c-2f573a5409f2
unrelated: b0e865b4-9204-4c55-9f5b-dcb755b3a03b, 902bd279-e6c1-4a21-9637-a3a43f360bae, aa275fac-def5-42a2-8a61-660142acad8e, b3cb190d-d6f1-4930-bdcd-7827e31ac62a, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, c6da863d-5085-402f-815a-9bb832b4df7e, dfd32ef6-c2ac-40ef-b8b3-9c58732516b1, aefa7f3c-70c3-412e-8d73-16ff9e2b703a, 61e73795-63de-4669-b09f-48be96919325, aa475aa3-05b2-429a-b33a-46b1dcb52049, 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f, aef1b384-2b33-47a2-92a8-ec7f8fb258ef, 75075c78-fe3c-4fd8-8a33-29a5913d9d34, c3127af2-4c96-48d5-a164-bac49a9a09ec, 38216d39-b064-4927-ab60-951a3c89e388, fe14991d-9715-4ae0-9a23-d314a950d3b9, aac5c313-dedd-466b-a0c3-ead5e99ce913, 848e1b76-9cfe-483b-94dd-094eb9b2f995, 069a6b4a-de35-45f3-8c6c-8e254bf40565, 3fc35d4f-90f3-4425-9dac-951a89631fdf, bceceda8-133a-4c9b-b439-b5c1a4414a02, 36dfbeaa-698d-4a57-b3b0-d425348e14a5, c84a4bd9-fc2d-44f0-b333-172978f61544, 6addc2f4-ff23-4fc6-851e-7cd67063b9f8, 7857d633-2055-47eb-857f-cb70da9fa7ba, d981f6b0-df0d-428f-8c0e-e220b1651fa1
id: f53ac3b9-efde-427c-8027-19988aa8fda7
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, blame file=108053a8-c2c3-4561-874c-2f573a5409f2, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, status=209081f4-864f-4ef6-8909-65da87f135e5, statuses=209081f4-864f-4ef6-8909-65da87f135e5, term=f590059d-b681-40ec-8a1c-c8916d71f6bb, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: the statuses of a question are one list in the code, and the statuses of a topic file, a term and a blame file are words each message picks for itself: new or changed, awaiting approval, not approved, removed.
Standard: a state machine with a closed set of statuses, each named once.

- A) No.
  Why: every thing's statuses are one closed list the tool holds, and every command prints a status by that name.
  Costs: a list per thing; a message that needs a status the list lacks waits for a decision.
- B) Yes.
  Why: each command describes what it sees in its own words.
  Costs: two commands name one status two ways, and an agent adds a status no decision gave.

**Answer:** A

### Does a command refuse what check passes?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, 0942e445-0163-423e-8ba7-84dd8ac07045, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, 3c8d4c9c-abca-4faa-b7c1-aae39951376c, b0e865b4-9204-4c55-9f5b-dcb755b3a03b, 48f4647d-8c81-4e8a-a936-6b32f5fb0699, 902bd279-e6c1-4a21-9637-a3a43f360bae, 4f04bae4-98e3-48eb-9403-3257268e2d87, 3ce82f34-a4fc-4918-bb9b-65a0d81a6853, d808917b-0717-45ad-ab83-9fa274b9a9d4, 05141f49-d9d2-407c-b60c-ebb195a05864, 7cf81ded-be48-4777-b83e-f8a2998c1ab5, aa475aa3-05b2-429a-b33a-46b1dcb52049
unrelated: 751ef30c-6a2f-4936-bf9b-b9d487146e53, 154e4b9a-c811-4a84-a522-598914eb39f1, 06b7e003-4d9f-427a-a798-c1af97865411, 6ef9eca6-fd8b-4211-aae7-00b37a2a9245, c3127af2-4c96-48d5-a164-bac49a9a09ec, b782664f-7465-4e1e-99e0-13f6dd2de774, f53ac3b9-efde-427c-8027-19988aa8fda7, aac5c313-dedd-466b-a0c3-ead5e99ce913, 848e1b76-9cfe-483b-94dd-094eb9b2f995, 069a6b4a-de35-45f3-8c6c-8e254bf40565, 3fc35d4f-90f3-4425-9dac-951a89631fdf, bceceda8-133a-4c9b-b439-b5c1a4414a02, 36dfbeaa-698d-4a57-b3b0-d425348e14a5, c84a4bd9-fc2d-44f0-b333-172978f61544, 6addc2f4-ff23-4fc6-851e-7cd67063b9f8, 7857d633-2055-47eb-857f-cb70da9fa7ba, 6108b8b2-911e-443b-abe0-4c6f8b407384, 9c3cc17e-33f4-4855-b761-f8f82381cdca
id: fe14991d-9715-4ae0-9a23-d314a950d3b9
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ban=ef83beae-6857-4200-b251-a0fcc3c72f9d, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, move=d904dc0d-231f-4e93-ac74-7babe888107d, refusal=0942e445-0163-423e-8ba7-84dd8ac07045, refuse=0942e445-0163-423e-8ba7-84dd8ac07045, refused=0942e445-0163-423e-8ba7-84dd8ac07045, refuses=0942e445-0163-423e-8ba7-84dd8ac07045

Problem: approve refuses a draft with a refused word that check passes, and refuses a removed file that check and show report as awaiting approval; each command judges the records on its own.
Standard: one judgement per rule on the records, read by every command that acts on it.

- A) No.
  Why: every rule is judged once and read by every judging command; what approve refuses, check reports with the same move.
  Costs: check reads every rule those commands read; a rule on the input of one command, like ban, stays there.
- B) Yes.
  Why: each command judges what it needs when it writes.
  Costs: a draft passes check and the commit, then stops at approve, and the agent learns the rule from the refusal.

**Answer:** A

### Does a command compute a move of its own?
depends: d904dc0d-231f-4e93-ac74-7babe888107d, 209081f4-864f-4ef6-8909-65da87f135e5, f53ac3b9-efde-427c-8027-19988aa8fda7=*, 48f4647d-8c81-4e8a-a936-6b32f5fb0699
unrelated: 902bd279-e6c1-4a21-9637-a3a43f360bae, dfd32ef6-c2ac-40ef-b8b3-9c58732516b1, aefa7f3c-70c3-412e-8d73-16ff9e2b703a, d3be71e4-ce7b-43b0-86a8-e86868521f99, 98d5d389-8315-41da-9970-666e31f3a94d, bc4291c8-98f1-499d-8e02-46e371035e70, 61e73795-63de-4669-b09f-48be96919325, f422c5db-13d0-4ff7-9e43-b68ab44a7cb9, b0e865b4-9204-4c55-9f5b-dcb755b3a03b, c3127af2-4c96-48d5-a164-bac49a9a09ec, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, ce19c883-c61d-477b-a6d4-fee5134c6e9a, 23d53545-c238-48cc-ae5e-21a2c7aab83e, ae63df95-0643-4108-af65-1e7a0009d631, fe14991d-9715-4ae0-9a23-d314a950d3b9, 848e1b76-9cfe-483b-94dd-094eb9b2f995, 069a6b4a-de35-45f3-8c6c-8e254bf40565, 3fc35d4f-90f3-4425-9dac-951a89631fdf, bceceda8-133a-4c9b-b439-b5c1a4414a02, 36dfbeaa-698d-4a57-b3b0-d425348e14a5, c84a4bd9-fc2d-44f0-b333-172978f61544, 6addc2f4-ff23-4fc6-851e-7cd67063b9f8, 7857d633-2055-47eb-857f-cb70da9fa7ba
id: aac5c313-dedd-466b-a0c3-ead5e99ce913
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answered=b36113a1-1384-4e4c-a798-8399f4181366, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, move=d904dc0d-231f-4e93-ac74-7babe888107d, moves=d904dc0d-231f-4e93-ac74-7babe888107d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, status=209081f4-864f-4ef6-8909-65da87f135e5

Problem: plan computes its next line on its own and names the owner's approval on a file approved, asked or answered; show computes the move elsewhere, and the two disagree.
Standard: one function per fact, every caller reads it.

- A) No.
  Why: one computation of the move from the status, printed the same by every command that prints one.
  Costs: plan's step lines come from that computation too, and a test holds every command to it.
- B) Yes.
  Why: each command knows its own step best.
  Costs: two commands name two moves for one status, and the agent follows the wrong one.

**Answer:** A

### What does a command that writes nothing print?
depends: 209081f4-864f-4ef6-8909-65da87f135e5, d904dc0d-231f-4e93-ac74-7babe888107d, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, f53ac3b9-efde-427c-8027-19988aa8fda7=*, aac5c313-dedd-466b-a0c3-ead5e99ce913=*, 05141f49-d9d2-407c-b60c-ebb195a05864, 2db33118-82b4-4edf-a9cb-e92ac62738b9, df452a5a-1e2d-497e-93e9-7d6a80e8540d, 99619e27-19b6-47ef-83b9-c79ac30ee2f0, 3460d220-e523-448b-aeff-baa8051b7d06
unrelated: 4f04bae4-98e3-48eb-9403-3257268e2d87, 902bd279-e6c1-4a21-9637-a3a43f360bae, f422c5db-13d0-4ff7-9e43-b68ab44a7cb9, aefa7f3c-70c3-412e-8d73-16ff9e2b703a, dfd32ef6-c2ac-40ef-b8b3-9c58732516b1, bf27b020-4e60-48e0-8d3b-5f818e6bf6d8, d8a9dfff-de89-442c-a08c-8276efab3f36, 34e81f57-9b3c-4dc4-a2ba-ca227523177d, 6108b8b2-911e-443b-abe0-4c6f8b407384, fe14991d-9715-4ae0-9a23-d314a950d3b9, 069a6b4a-de35-45f3-8c6c-8e254bf40565, 3fc35d4f-90f3-4425-9dac-951a89631fdf, bceceda8-133a-4c9b-b439-b5c1a4414a02, 36dfbeaa-698d-4a57-b3b0-d425348e14a5, c84a4bd9-fc2d-44f0-b333-172978f61544, 6addc2f4-ff23-4fc6-851e-7cd67063b9f8, 7857d633-2055-47eb-857f-cb70da9fa7ba, 154e4b9a-c811-4a84-a522-598914eb39f1, 75075c78-fe3c-4fd8-8a33-29a5913d9d34
id: 848e1b76-9cfe-483b-94dd-094eb9b2f995
terms: check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, move=d904dc0d-231f-4e93-ac74-7babe888107d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, status=209081f4-864f-4ef6-8909-65da87f135e5, statuses=209081f4-864f-4ef6-8909-65da87f135e5

Problem: show, check, plan and read each print lines the one who runs them has no move on, and the move hides among them.
Standard: a read-only command prints what it read and what follows from it, as git status does.

- A) What it reads from the records, its status, the findings, and the move that follows, and nothing else.
  Why: every line is read for a move; the record names each actor's inputs at each step.
  Costs: counts, pass notes and the hint line go; a passing check prints nothing.
- B) Everything it computed on the way: statuses, findings, counts, notes and the move.
  Why: the one who runs it sees that the command ran and what it covered.
  Costs: hundreds of lines around one move; each new command adds lines no decision names.

**Answer:** A

### What does a command that writes print?
depends: e18b82d8-5271-4385-bbab-2b89e5da278f, 0942e445-0163-423e-8ba7-84dd8ac07045, d904dc0d-231f-4e93-ac74-7babe888107d, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, 3c8d4c9c-abca-4faa-b7c1-aae39951376c, b36113a1-1384-4e4c-a798-8399f4181366, ef83beae-6857-4200-b251-a0fcc3c72f9d, fe14991d-9715-4ae0-9a23-d314a950d3b9=*, 4f04bae4-98e3-48eb-9403-3257268e2d87, f422c5db-13d0-4ff7-9e43-b68ab44a7cb9, ae63df95-0643-4108-af65-1e7a0009d631
unrelated: aefa7f3c-70c3-412e-8d73-16ff9e2b703a, dfd32ef6-c2ac-40ef-b8b3-9c58732516b1, 99619e27-19b6-47ef-83b9-c79ac30ee2f0, 26c5352f-3f82-4376-9dc2-8ccc0074d080, db56a285-946f-48b1-b073-bb1e90192185, 385a57d5-5f76-454a-ae5d-9b0d6f5c1492, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, 154e4b9a-c811-4a84-a522-598914eb39f1, 2db33118-82b4-4edf-a9cb-e92ac62738b9, 902bd279-e6c1-4a21-9637-a3a43f360bae, 7b05f8df-09a5-404e-a0f0-3dd8824d1d15, 75075c78-fe3c-4fd8-8a33-29a5913d9d34, 97c679f6-e8e6-4970-b547-940477753d7d, 05141f49-d9d2-407c-b60c-ebb195a05864, c3127af2-4c96-48d5-a164-bac49a9a09ec, 21ae8ae0-6884-4d3c-8464-024a8a5aae2d, 11be073c-ce91-44d8-833a-ef0b5e0b1672, f53ac3b9-efde-427c-8027-19988aa8fda7, aac5c313-dedd-466b-a0c3-ead5e99ce913, 848e1b76-9cfe-483b-94dd-094eb9b2f995, 3fc35d4f-90f3-4425-9dac-951a89631fdf, bceceda8-133a-4c9b-b439-b5c1a4414a02, 36dfbeaa-698d-4a57-b3b0-d425348e14a5, c84a4bd9-fc2d-44f0-b333-172978f61544, 6addc2f4-ff23-4fc6-851e-7cd67063b9f8, 7857d633-2055-47eb-857f-cb70da9fa7ba
id: 069a6b4a-de35-45f3-8c6c-8e254bf40565
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, answers=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, ban=ef83beae-6857-4200-b251-a0fcc3c72f9d, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, move=d904dc0d-231f-4e93-ac74-7babe888107d, moves=d904dc0d-231f-4e93-ac74-7babe888107d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c

Problem: approve, answer and ask print tier lines and the lines of other commands, install and the ban rebuild print lines no decision names, and what the owner reads after an act has no decision behind it.
Standard: a command that writes prints one line per event, as git commit does, or nothing, as mv does.

- A) The event it wrote, one line each, or the finding that stopped it, and nothing else.
  Why: the one who runs it reads what changed; the move after it comes from show.
  Costs: the move after an act takes a second command.
- B) Nothing on success; the finding that stopped it on a stop.
  Why: silence is success.
  Costs: an owner who runs approve sees no sign of what was approved.
- C) The event it wrote and the moves that follow it.
  Why: one command gives the agent its next move.
  Costs: ask repeats what show prints, and the owner's answer lines reach a terminal the owner answers from the page.

**Answer:** A

### Does check print a line that names no move?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, d904dc0d-231f-4e93-ac74-7babe888107d, 848e1b76-9cfe-483b-94dd-094eb9b2f995=*, aac5c313-dedd-466b-a0c3-ead5e99ce913=*, 3460d220-e523-448b-aeff-baa8051b7d06, 05141f49-d9d2-407c-b60c-ebb195a05864
supersedes: 154e4b9a-c811-4a84-a522-598914eb39f1, d981f6b0-df0d-428f-8c0e-e220b1651fa1, 9c3cc17e-33f4-4855-b761-f8f82381cdca
unrelated: 6108b8b2-911e-443b-abe0-4c6f8b407384, 4f04bae4-98e3-48eb-9403-3257268e2d87, c10c3a49-e993-4c9f-a46b-4a1ba3646dcf, 751ef30c-6a2f-4936-bf9b-b9d487146e53, a82db5b7-969c-44e9-9eec-876ead7099d3, 902bd279-e6c1-4a21-9637-a3a43f360bae, bf6697e3-8832-4b90-b036-8fecedb55406, 75075c78-fe3c-4fd8-8a33-29a5913d9d34, 34e81f57-9b3c-4dc4-a2ba-ca227523177d, f53ac3b9-efde-427c-8027-19988aa8fda7, fe14991d-9715-4ae0-9a23-d314a950d3b9, 069a6b4a-de35-45f3-8c6c-8e254bf40565, bceceda8-133a-4c9b-b439-b5c1a4414a02, 36dfbeaa-698d-4a57-b3b0-d425348e14a5, c84a4bd9-fc2d-44f0-b333-172978f61544, 6addc2f4-ff23-4fc6-851e-7cd67063b9f8, 7857d633-2055-47eb-857f-cb70da9fa7ba
id: 3fc35d4f-90f3-4425-9dac-951a89631fdf
terms: approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, blame file=108053a8-c2c3-4561-874c-2f573a5409f2, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, move=d904dc0d-231f-4e93-ac74-7babe888107d, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: on a pass check prints a tier line and a done line per topic file and counts of tested decisions and files awaiting approval, and none carries a move.
Standard: a checker prints its findings, each with the fix, and is silent on a pass.

- A) No.
  Why: every line of check is a finding with its move; a pass prints nothing, by hand or in a hook.
  Costs: hook flag, done, tier and tested lines go; no decision holds that a waiting blame file stops nothing.
- B) Yes.
  Why: the one who runs check sees what it covered.
  Costs: dozens of lines with no move on every run, and twice per commit without a flag.

**Answer:** A

### Does show print what a file holds beyond its latest approval?
depends: e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, 1d7e59ea-99e3-4d4b-9928-5dabca62723d, 209081f4-864f-4ef6-8909-65da87f135e5, 08193e75-dd89-4857-8e9f-b19e1a9e75e6, 528fe6f6-6d77-42d5-9086-017048d1448d, 108053a8-c2c3-4561-874c-2f573a5409f2, f53ac3b9-efde-427c-8027-19988aa8fda7=*, 848e1b76-9cfe-483b-94dd-094eb9b2f995=*, aa475aa3-05b2-429a-b33a-46b1dcb52049, e879fe72-ec07-48cd-9902-2b158aaae726
unrelated: 3ce82f34-a4fc-4918-bb9b-65a0d81a6853, 05141f49-d9d2-407c-b60c-ebb195a05864, 912f4f3b-1213-435a-af0d-7617017fcff4, 9c3cc17e-33f4-4855-b761-f8f82381cdca, 154e4b9a-c811-4a84-a522-598914eb39f1, 26c5352f-3f82-4376-9dc2-8ccc0074d080, 4bb5ef1b-da16-4206-a7d2-c7380c36a4cc, 34e81f57-9b3c-4dc4-a2ba-ca227523177d, 11be073c-ce91-44d8-833a-ef0b5e0b1672, 644b43ce-be10-4e9a-a7d5-71b27c51e365, d981f6b0-df0d-428f-8c0e-e220b1651fa1, fe14991d-9715-4ae0-9a23-d314a950d3b9, aac5c313-dedd-466b-a0c3-ead5e99ce913, 069a6b4a-de35-45f3-8c6c-8e254bf40565, 3fc35d4f-90f3-4425-9dac-951a89631fdf, 36dfbeaa-698d-4a57-b3b0-d425348e14a5, c84a4bd9-fc2d-44f0-b333-172978f61544, 6addc2f4-ff23-4fc6-851e-7cd67063b9f8, 7857d633-2055-47eb-857f-cb70da9fa7ba, 75075c78-fe3c-4fd8-8a33-29a5913d9d34, 7b05f8df-09a5-404e-a0f0-3dd8824d1d15
id: bceceda8-133a-4c9b-b439-b5c1a4414a02
terms: approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, blame=108053a8-c2c3-4561-874c-2f573a5409f2, blame file=108053a8-c2c3-4561-874c-2f573a5409f2, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, refuses=0942e445-0163-423e-8ba7-84dd8ac07045, status=209081f4-864f-4ef6-8909-65da87f135e5, terms file=528fe6f6-6d77-42d5-9086-017048d1448d, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: the owner runs approve without a view of what it approves: show prints a topic file the same before and after approval, never its bodies, and refuses the terms file and a blame path.
Standard: a diff before a commit: the change is read before it is signed.

- A) Yes.
  Why: show takes every file approve takes and prints the headings, lines, bodies, entries or rows beyond its latest approval, marked.
  Costs: show parses the terms file and a blame file; a plan change prints the change, not the file.
- B) No.
  Why: show prints status, and a draft is read with git diff.
  Costs: the owner leaves the tool to see what approve will sign.

**Answer:** A

### Does a command print under a flag what it computes from the records?
depends: daec9ffd-676f-4111-a359-2827bfd4527c, be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, d904dc0d-231f-4e93-ac74-7babe888107d, 848e1b76-9cfe-483b-94dd-094eb9b2f995=*, 2db33118-82b4-4edf-a9cb-e92ac62738b9, 11be073c-ce91-44d8-833a-ef0b5e0b1672
supersedes: 97c679f6-e8e6-4970-b547-940477753d7d
unrelated: 154e4b9a-c811-4a84-a522-598914eb39f1, 75075c78-fe3c-4fd8-8a33-29a5913d9d34, aefa7f3c-70c3-412e-8d73-16ff9e2b703a, 902bd279-e6c1-4a21-9637-a3a43f360bae, 385a57d5-5f76-454a-ae5d-9b0d6f5c1492, 05141f49-d9d2-407c-b60c-ebb195a05864, 34e81f57-9b3c-4dc4-a2ba-ca227523177d, 99619e27-19b6-47ef-83b9-c79ac30ee2f0, d8a9dfff-de89-442c-a08c-8276efab3f36, f53ac3b9-efde-427c-8027-19988aa8fda7, fe14991d-9715-4ae0-9a23-d314a950d3b9, aac5c313-dedd-466b-a0c3-ead5e99ce913, 069a6b4a-de35-45f3-8c6c-8e254bf40565, 3fc35d4f-90f3-4425-9dac-951a89631fdf, bceceda8-133a-4c9b-b439-b5c1a4414a02, c84a4bd9-fc2d-44f0-b333-172978f61544, 6addc2f4-ff23-4fc6-851e-7cd67063b9f8, 7857d633-2055-47eb-857f-cb70da9fa7ba, 7b05f8df-09a5-404e-a0f0-3dd8824d1d15
id: 36dfbeaa-698d-4a57-b3b0-d425348e14a5
terms: decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, move=d904dc0d-231f-4e93-ac74-7babe888107d, neighbours=daec9ffd-676f-4111-a359-2827bfd4527c, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, reader=be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, status=209081f4-864f-4ef6-8909-65da87f135e5

Problem: show prints the neighbours of every unasked question with the reader's word in its plain listing, five lines per heading that no move reads.
Standard: a computed view is printed on request, under a flag named for it, as git log --graph and ls -l do.

- A) Yes.
  Why: status and move in the listing; a computed view under a flag, or inside a finding or move needing it.
  Costs: a flag per view; a view with no flag is read from the files the tool writes.
- B) No.
  Why: a command prints the status and the move, and nothing computed, flag or not.
  Costs: the nearest decisions and the reader's word are read from plan's output or the reader's file alone.

**Answer:** A

### What does show print with no file named?
depends: 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, e2533580-1486-4dc9-8697-e6c53605fe72, 43778946-d44c-4acf-a766-0a4915fce199, d904dc0d-231f-4e93-ac74-7babe888107d, 848e1b76-9cfe-483b-94dd-094eb9b2f995=*, 36dfbeaa-698d-4a57-b3b0-d425348e14a5=*
supersedes: 75075c78-fe3c-4fd8-8a33-29a5913d9d34, 7b05f8df-09a5-404e-a0f0-3dd8824d1d15
unrelated: 154e4b9a-c811-4a84-a522-598914eb39f1, 4f04bae4-98e3-48eb-9403-3257268e2d87, 05141f49-d9d2-407c-b60c-ebb195a05864, 0d977e39-a657-4596-8eca-8ceb335f3081, 2db33118-82b4-4edf-a9cb-e92ac62738b9, 4bb5ef1b-da16-4206-a7d2-c7380c36a4cc, 3ce82f34-a4fc-4918-bb9b-65a0d81a6853, 97c679f6-e8e6-4970-b547-940477753d7d, c3127af2-4c96-48d5-a164-bac49a9a09ec, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, 11be073c-ce91-44d8-833a-ef0b5e0b1672, b0e865b4-9204-4c55-9f5b-dcb755b3a03b, ae63df95-0643-4108-af65-1e7a0009d631, 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f, f53ac3b9-efde-427c-8027-19988aa8fda7, fe14991d-9715-4ae0-9a23-d314a950d3b9, aac5c313-dedd-466b-a0c3-ead5e99ce913, 069a6b4a-de35-45f3-8c6c-8e254bf40565, 3fc35d4f-90f3-4425-9dac-951a89631fdf, bceceda8-133a-4c9b-b439-b5c1a4414a02, 6addc2f4-ff23-4fc6-851e-7cd67063b9f8, 7857d633-2055-47eb-857f-cb70da9fa7ba
id: c84a4bd9-fc2d-44f0-b333-172978f61544
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, move=d904dc0d-231f-4e93-ac74-7babe888107d, moves=d904dc0d-231f-4e93-ac74-7babe888107d, option=43778946-d44c-4acf-a766-0a4915fce199, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, status=209081f4-864f-4ef6-8909-65da87f135e5

Problem: an agent runs show to find its move and to read the decisions it builds on; a listing of everything runs to hundreds of lines, and a listing of moves alone leaves the decisions to a second form.
Standard: a listing command prints a summary and takes a flag or an argument for the whole.

- A) Every current decision as its heading and the option picked, and every move.
  Why: the agent reads the move first and the decisions it builds on second, in one run.
  Costs: a few lines per decision; the file form and the whole are a second run.
- B) Every move, and the decisions under a flag.
  Why: the shortest listing; the decisions come when asked for.
  Costs: the agent runs show twice before it builds.
- C) Every question of every file with its status and ties.
  Why: one listing holds everything.
  Costs: hundreds of lines to find one move, growing with every question.

**Answer:** A

### What does show print with a file named?
depends: 209081f4-864f-4ef6-8909-65da87f135e5, 277a8338-c57a-43cc-8b28-6b42e9347f6a, f590059d-b681-40ec-8a1c-c8916d71f6bb, 08193e75-dd89-4857-8e9f-b19e1a9e75e6, 528fe6f6-6d77-42d5-9086-017048d1448d, 108053a8-c2c3-4561-874c-2f573a5409f2, 3c8d4c9c-abca-4faa-b7c1-aae39951376c, 644b43ce-be10-4e9a-a7d5-71b27c51e365, bceceda8-133a-4c9b-b439-b5c1a4414a02=*, 36dfbeaa-698d-4a57-b3b0-d425348e14a5=*, 848e1b76-9cfe-483b-94dd-094eb9b2f995=*, 38216d39-b064-4927-ab60-951a3c89e388, 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f, 11be073c-ce91-44d8-833a-ef0b5e0b1672, 34f0202a-0be4-4049-94de-d3886f4f8ffc, f53ac3b9-efde-427c-8027-19988aa8fda7=*
supersedes: 75075c78-fe3c-4fd8-8a33-29a5913d9d34, 7b05f8df-09a5-404e-a0f0-3dd8824d1d15
unrelated: 154e4b9a-c811-4a84-a522-598914eb39f1, 2db33118-82b4-4edf-a9cb-e92ac62738b9, 08a1b488-32fc-4dc7-b4b3-93a6e768725f, 99619e27-19b6-47ef-83b9-c79ac30ee2f0, 05141f49-d9d2-407c-b60c-ebb195a05864, d981f6b0-df0d-428f-8c0e-e220b1651fa1, 4bb5ef1b-da16-4206-a7d2-c7380c36a4cc, 4f04bae4-98e3-48eb-9403-3257268e2d87, fe14991d-9715-4ae0-9a23-d314a950d3b9, aac5c313-dedd-466b-a0c3-ead5e99ce913, 069a6b4a-de35-45f3-8c6c-8e254bf40565, 3fc35d4f-90f3-4425-9dac-951a89631fdf, c84a4bd9-fc2d-44f0-b333-172978f61544, 7857d633-2055-47eb-857f-cb70da9fa7ba, 97c679f6-e8e6-4970-b547-940477753d7d
id: 6addc2f4-ff23-4fc6-851e-7cd67063b9f8
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, answered=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, blame=108053a8-c2c3-4561-874c-2f573a5409f2, blame file=108053a8-c2c3-4561-874c-2f573a5409f2, move=d904dc0d-231f-4e93-ac74-7babe888107d, neighbours=daec9ffd-676f-4111-a359-2827bfd4527c, options=43778946-d44c-4acf-a766-0a4915fce199, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, refuses=0942e445-0163-423e-8ba7-84dd8ac07045, stands on=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, status=209081f4-864f-4ef6-8909-65da87f135e5, terms file=528fe6f6-6d77-42d5-9086-017048d1448d, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: show with a topic file prints each question with its status, its lines and its neighbours, and no body; show refuses the terms file and a blame path, so two of the three drafts the owner approves are read with git.
Standard: a listing command takes every path the writing command takes and prints the record held at that path.

- A) Every question, entry or row with its status, the lines it carries and the body of an asked question.
  Why: one form for every file approve takes; the owner reads the options where the move is to answer.
  Costs: a large blame file prints every row; what stands on a question is a computed view.
- B) Every question, entry or row with its status, the lines it carries and its body.
  Why: the whole file in one run, options and all.
  Costs: a file of twenty answered questions prints hundreds of lines to find one status.

**Answer:** A

### Does the ui show what the terminal prints?
depends: 7127ab1e-c313-4c51-9134-c09d551682cf, 1d7e59ea-99e3-4d4b-9928-5dabca62723d, 848e1b76-9cfe-483b-94dd-094eb9b2f995=*, 069a6b4a-de35-45f3-8c6c-8e254bf40565=*, c84a4bd9-fc2d-44f0-b333-172978f61544=*, 644b43ce-be10-4e9a-a7d5-71b27c51e365, 11be073c-ce91-44d8-833a-ef0b5e0b1672, ece60954-56d4-4622-883e-265f3b997c9f
unrelated: 7b05f8df-09a5-404e-a0f0-3dd8824d1d15, 75075c78-fe3c-4fd8-8a33-29a5913d9d34, 2db33118-82b4-4edf-a9cb-e92ac62738b9, 154e4b9a-c811-4a84-a522-598914eb39f1, b3cb190d-d6f1-4930-bdcd-7827e31ac62a, 97c679f6-e8e6-4970-b547-940477753d7d, 0487a1cf-eb61-4bf8-bd16-eed899d22d2c, aa275fac-def5-42a2-8a61-660142acad8e, c3127af2-4c96-48d5-a164-bac49a9a09ec, 05141f49-d9d2-407c-b60c-ebb195a05864, 3460d220-e523-448b-aeff-baa8051b7d06, f53ac3b9-efde-427c-8027-19988aa8fda7, fe14991d-9715-4ae0-9a23-d314a950d3b9, aac5c313-dedd-466b-a0c3-ead5e99ce913, 3fc35d4f-90f3-4425-9dac-951a89631fdf, bceceda8-133a-4c9b-b439-b5c1a4414a02, 36dfbeaa-698d-4a57-b3b0-d425348e14a5, 6addc2f4-ff23-4fc6-851e-7cd67063b9f8
id: 7857d633-2055-47eb-857f-cb70da9fa7ba
terms: check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, moves=d904dc0d-231f-4e93-ac74-7babe888107d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, statuses=209081f4-864f-4ef6-8909-65da87f135e5, ui=7127ab1e-c313-4c51-9134-c09d551682cf

Problem: the ui copies check's whole output into a block, shows one open question at a time with no count of the rest, and its lines have no decision behind them apart from the owner's acts.
Standard: one view model, two renderings.

- A) Yes.
  Why: the page renders the same statuses, findings, moves and events the commands print, from the one computation.
  Costs: the page shows nothing the terminal cannot; a page form for every line kind.
- B) No.
  Why: the page is laid out for the owner alone, with its own lines.
  Costs: two sets of lines to hold right, and a line on the page with no decision behind it.

**Answer:** A

### What does show print of the file named?
depends: 209081f4-864f-4ef6-8909-65da87f135e5, 277a8338-c57a-43cc-8b28-6b42e9347f6a, f590059d-b681-40ec-8a1c-c8916d71f6bb, 08193e75-dd89-4857-8e9f-b19e1a9e75e6, 528fe6f6-6d77-42d5-9086-017048d1448d, 108053a8-c2c3-4561-874c-2f573a5409f2, 3c8d4c9c-abca-4faa-b7c1-aae39951376c, 644b43ce-be10-4e9a-a7d5-71b27c51e365, bceceda8-133a-4c9b-b439-b5c1a4414a02, 36dfbeaa-698d-4a57-b3b0-d425348e14a5, 848e1b76-9cfe-483b-94dd-094eb9b2f995, 38216d39-b064-4927-ab60-951a3c89e388, 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f, 11be073c-ce91-44d8-833a-ef0b5e0b1672, 34f0202a-0be4-4049-94de-d3886f4f8ffc, f53ac3b9-efde-427c-8027-19988aa8fda7
supersedes: 6addc2f4-ff23-4fc6-851e-7cd67063b9f8
unrelated: c84a4bd9-fc2d-44f0-b333-172978f61544, 7857d633-2055-47eb-857f-cb70da9fa7ba, 069a6b4a-de35-45f3-8c6c-8e254bf40565, 3fc35d4f-90f3-4425-9dac-951a89631fdf, 99619e27-19b6-47ef-83b9-c79ac30ee2f0, 385a57d5-5f76-454a-ae5d-9b0d6f5c1492, 6b3d8e59-e145-49e1-860d-a79d753d745c, 184b5bcb-fa72-436b-92cb-4d4605366c59, 110f471d-ac4b-4cbb-aa7b-3f74ac442d78
id: dce141e9-7d2e-4b70-b317-9f3d70bf5a69
terms: answers=b36113a1-1384-4e4c-a798-8399f4181366, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, move=d904dc0d-231f-4e93-ac74-7babe888107d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, status=209081f4-864f-4ef6-8909-65da87f135e5, supersedes=2bae531b-c4df-48a0-89a0-3d52726741ea, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: show with a topic file prints the unrelated line of every question, twenty names each and over half the output, and no move reads that line after approval.
Standard: a listing prints what the next move reads and takes a flag for the rest.

- A) Every question, entry or row with its status, every depends and supersedes line, and the body of an asked question.
  Why: every printed line is read for a move: the owner approves on the lines and answers on the bodies.
  Costs: the unrelated line, changed or not, comes under a flag named for it.
- B) Every question, entry or row with its status, every line it carries and the body of an asked question.
  Why: the whole file in one run, nothing under a flag.
  Costs: twenty names per question that no move reads, over half the output of a planned file.

**Answer:** A

### What does a finding name?
depends: ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, d904dc0d-231f-4e93-ac74-7babe888107d, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, f90dbd5b-8fbe-4899-8588-f8762795c2e2, b0e865b4-9204-4c55-9f5b-dcb755b3a03b, 48f4647d-8c81-4e8a-a936-6b32f5fb0699, fe14991d-9715-4ae0-9a23-d314a950d3b9, 848e1b76-9cfe-483b-94dd-094eb9b2f995, 069a6b4a-de35-45f3-8c6c-8e254bf40565, 3fc35d4f-90f3-4425-9dac-951a89631fdf, aac5c313-dedd-466b-a0c3-ead5e99ce913
unrelated: 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f, 7c306235-6243-4d79-b9aa-08ed19662779, 4319a653-6901-446c-94ab-6420ef472134, ad9761e2-6d64-4b45-b0ba-5c5d8d2b38bf, 38216d39-b064-4927-ab60-951a3c89e388, 6addc2f4-ff23-4fc6-851e-7cd67063b9f8, aef1b384-2b33-47a2-92a8-ec7f8fb258ef, adee66f2-c8ff-477f-8ac4-a497f694c912, c84a4bd9-fc2d-44f0-b333-172978f61544, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, c3127af2-4c96-48d5-a164-bac49a9a09ec, 6108b8b2-911e-443b-abe0-4c6f8b407384, 110f471d-ac4b-4cbb-aa7b-3f74ac442d78, dce141e9-7d2e-4b70-b317-9f3d70bf5a69
id: 184b5bcb-fa72-436b-92cb-4d4605366c59
terms: answered=b36113a1-1384-4e4c-a798-8399f4181366, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, finding=ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, identifiers=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, move=d904dc0d-231f-4e93-ac74-7babe888107d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d

Problem: a finding names the number it measured and the act that lowers the number; the one who reads it clears the number, and the meaning the rule protects changes.
Standard: a compiler error names the rule a line breaks and leaves the fix to the writer.

- A) What the rule protects, in plain words, what it found, and the move as an act, not a diagnosis.
  Why: the one who reads it sees what the rule protects and rewrites; the tool never guesses the cause.
  Costs: the rule's words are the writer's, not the record's, and review alone holds them to it.
- B) The decision it judges by heading, what it found, and the move.
  Why: the record itself names the rule, in the words the owner answered.
  Costs: a terminal in any repository prints the headings and identifiers of this tool's own record.
- C) What it measured and the act that clears the measurement.
  Why: the shortest line, and the one the counter gives for free.
  Costs: the move names the counter, not the rule, and the fix that clears the counter can change the meaning.

**Answer:** A

### Does a finding carry the identifier of the decision it judges?
depends: ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, e877e4a9-2caf-4fe6-9f82-5476a2abb00b, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, e2533580-1486-4dc9-8697-e6c53605fe72, 4319a653-6901-446c-94ab-6420ef472134, 751ef30c-6a2f-4936-bf9b-b9d487146e53, fe14991d-9715-4ae0-9a23-d314a950d3b9, 184b5bcb-fa72-436b-92cb-4d4605366c59=A|B, 7a1e7b81-0967-4ed2-8a8a-c7153056836f, c3127af2-4c96-48d5-a164-bac49a9a09ec
unrelated: de5ed5b1-3a83-4403-8953-941cb7e94390, 946263aa-519a-47aa-b743-5cdcf52f5d35, 4b407ee1-1e64-4ca1-842b-69773fc0e48a, 2bae531b-c4df-48a0-89a0-3d52726741ea, 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, 14180eab-60a4-48d3-8efc-c1b23a96ce3a, 3460d220-e523-448b-aeff-baa8051b7d06, 6108b8b2-911e-443b-abe0-4c6f8b407384, c6da863d-5085-402f-815a-9bb832b4df7e, 7c306235-6243-4d79-b9aa-08ed19662779, dce141e9-7d2e-4b70-b317-9f3d70bf5a69
id: 110f471d-ac4b-4cbb-aa7b-3f74ac442d78
terms: current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, finding=ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, findings=ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, superseded=2bae531b-c4df-48a0-89a0-3d52726741ea

Problem: a finding is tagged with a name the code picks, tells, plan, ids; nothing ties a message to the decision it judges, and a message with no decision behind it is written like the rest.
Standard: every diagnostic carries a stable code that documentation and tests key on.

- A) Yes.
  Why: a finding with no current decision behind it cannot be built; one test holds every finding to its decision.
  Costs: the identifier stays in the code and the test, never on the line; a superseded decision re-tags its findings.
- B) No.
  Why: the code names its rules as it likes; the record and the messages stay apart.
  Costs: a message with no decision behind it passes every gate and is caught at review alone.

**Answer:** A

### What does show print of a file under adrdom?
depends: d904dc0d-231f-4e93-ac74-7babe888107d, 209081f4-864f-4ef6-8909-65da87f135e5, 36dfbeaa-698d-4a57-b3b0-d425348e14a5, bceceda8-133a-4c9b-b439-b5c1a4414a02
supersedes: dce141e9-7d2e-4b70-b317-9f3d70bf5a69
unrelated: c84a4bd9-fc2d-44f0-b333-172978f61544, 08a1b488-32fc-4dc7-b4b3-93a6e768725f, 16a71892-b348-4b9b-82c5-a9d375187bd3, 7857d633-2055-47eb-857f-cb70da9fa7ba, 7d44030a-3589-4f29-aaa6-12fa09908ab8, 08193e75-dd89-4857-8e9f-b19e1a9e75e6, e18b82d8-5271-4385-bbab-2b89e5da278f, 3fc35d4f-90f3-4425-9dac-951a89631fdf, 05141f49-d9d2-407c-b60c-ebb195a05864, 11be073c-ce91-44d8-833a-ef0b5e0b1672, 4319a653-6901-446c-94ab-6420ef472134
id: a6f41dee-2381-421a-bcc4-b14af243209b
terms: answers=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, move=d904dc0d-231f-4e93-ac74-7babe888107d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, status=209081f4-864f-4ef6-8909-65da87f135e5

Problem: show exists so that the one who runs it reads what their next move on the file needs, whatever kind of file it is.
Standard: a listing prints what the next move reads and takes a flag for the rest.

- A) What the next move on the file reads, and the rest under a flag.
  Why: a question with its status, the lines approve reads, an asked body; a new line or file kind changes nothing.
  Costs: which lines a move reads is derived from that move's decision, not listed in this one.
- B) Everything the file holds, with the status of each thing.
  Why: the whole file in one run, nothing under a flag.
  Costs: twenty unrelated names per question that no move reads, over half the output.
- C) Each thing's status only, and every line and body under a flag.
  Why: the shortest listing.
  Costs: the owner approves lines and answers bodies without seeing them.

**Answer:** A

### What does show print of the reader's word on a question?
depends: be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, 277a8338-c57a-43cc-8b28-6b42e9347f6a, daec9ffd-676f-4111-a359-2827bfd4527c, a6f41dee-2381-421a-bcc4-b14af243209b, 36dfbeaa-698d-4a57-b3b0-d425348e14a5, 2db33118-82b4-4edf-a9cb-e92ac62738b9, c6e0b7d6-cdec-46da-b2bc-cda218673868, 08a1b488-32fc-4dc7-b4b3-93a6e768725f
unrelated: 11be073c-ce91-44d8-833a-ef0b5e0b1672, 55a8abb2-0f6d-430b-99f9-a378f3f154e5, 6108b8b2-911e-443b-abe0-4c6f8b407384, f5c242b6-e2f6-45a3-8c61-c743f3ad794f, bceceda8-133a-4c9b-b439-b5c1a4414a02, 9d634996-475e-4ddc-8113-05c404ad9d5c, 21ae8ae0-6884-4d3c-8464-024a8a5aae2d, c3127af2-4c96-48d5-a164-bac49a9a09ec, aa475aa3-05b2-429a-b33a-46b1dcb52049, 644b43ce-be10-4e9a-a7d5-71b27c51e365, 05141f49-d9d2-407c-b60c-ebb195a05864, 83190b10-8060-47c1-8194-f6a5c4f0d703, b80e29dc-3f25-4aa8-8fc8-fa5ea57cd051, f3c9e736-157c-4197-b49e-19e3674e4e0c, 23d53545-c238-48cc-ae5e-21a2c7aab83e
id: 332ff0c7-0769-45ac-8f89-00aeff318467
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approving=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, move=d904dc0d-231f-4e93-ac74-7babe888107d, moves=d904dc0d-231f-4e93-ac74-7babe888107d, neighbour=daec9ffd-676f-4111-a359-2827bfd4527c, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, reader=be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, supersedes=2bae531b-c4df-48a0-89a0-3d52726741ea

Problem: the owner approves a plan reading the agent's lines, and the reader's word on those lines, inconsistent or same, is printed to the agent alone.
Standard: a review listing shows a disputed item beside the item, and the full analysis on request.

- A) Under the question, [inconsistent] or [same] with the neighbour when no supersedes line names it; every word under --reader.
  Why: the owner sees a disputed line beside the line before approving; the rest stays out of the plain listing.
  Costs: a view with no move on its line; show reads the reader's file for each question.
- B) Every word under --reader, nothing in the plain listing.
  Why: the plain listing stays the agent's lines and the moves.
  Costs: the owner runs the flag before each approval, or approves a disputed line unseen.
- C) Nothing: the reader's word is read in plan's output alone.
  Why: no new line form.
  Costs: the owner never sees the reader's word; the agent's line stands alone at approval.

**Answer:** A

### Does show print the reference lines of an answered question?
depends: aef1b384-2b33-47a2-92a8-ec7f8fb258ef, b36113a1-1384-4e4c-a798-8399f4181366, a6f41dee-2381-421a-bcc4-b14af243209b
unrelated: 946263aa-519a-47aa-b743-5cdcf52f5d35, 11be073c-ce91-44d8-833a-ef0b5e0b1672, 1fb71321-bb84-492f-b9d5-1444c9c444af, ad9761e2-6d64-4b45-b0ba-5c5d8d2b38bf, 55a8abb2-0f6d-430b-99f9-a378f3f154e5, 6108b8b2-911e-443b-abe0-4c6f8b407384, 462e22c6-8b23-4061-9a2a-bbf8691fce60, 7327cd58-ac58-40a4-8ef2-00072fb552b2, 9d634996-475e-4ddc-8113-05c404ad9d5c, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, bceceda8-133a-4c9b-b439-b5c1a4414a02, c84a4bd9-fc2d-44f0-b333-172978f61544, 332ff0c7-0769-45ac-8f89-00aeff318467, b80e29dc-3f25-4aa8-8fc8-fa5ea57cd051, f3c9e736-157c-4197-b49e-19e3674e4e0c, 36dfbeaa-698d-4a57-b3b0-d425348e14a5, a82db5b7-969c-44e9-9eec-876ead7099d3, 3fc35d4f-90f3-4425-9dac-951a89631fdf, 7857d633-2055-47eb-857f-cb70da9fa7ba, 0487a1cf-eb61-4bf8-bd16-eed899d22d2c, 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f
id: 83190b10-8060-47c1-8194-f6a5c4f0d703
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answered=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, blame lines=108053a8-c2c3-4561-874c-2f573a5409f2, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, move=d904dc0d-231f-4e93-ac74-7babe888107d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, supersedes=2bae531b-c4df-48a0-89a0-3d52726741ea

Problem: the depends, supersedes and blame lines of every answered question fill half of show's listing of a file, and no move on an answered question reads them.
Standard: a listing prints what the next step reads; what stands behind a closed item is shown on request.

- A) No.
  Why: the lines of an unanswered question are what approve reads; those of an answered one print under --dependents.
  Costs: an agent after an answered question's lines runs --dependents or reads the file.
- B) Yes.
  Why: every line of every question in one listing.
  Costs: lines no move reads make most of the listing of a settled file.

**Answer:** A

### Does the tool colour what it prints to a terminal?
depends: 1d7e59ea-99e3-4d4b-9928-5dabca62723d, f90dbd5b-8fbe-4899-8588-f8762795c2e2, 848e1b76-9cfe-483b-94dd-094eb9b2f995, 069a6b4a-de35-45f3-8c6c-8e254bf40565, 7857d633-2055-47eb-857f-cb70da9fa7ba
unrelated: 2db33118-82b4-4edf-a9cb-e92ac62738b9, 36dfbeaa-698d-4a57-b3b0-d425348e14a5, c84a4bd9-fc2d-44f0-b333-172978f61544, 3fc35d4f-90f3-4425-9dac-951a89631fdf, a6f41dee-2381-421a-bcc4-b14af243209b, f53ac3b9-efde-427c-8027-19988aa8fda7, bceceda8-133a-4c9b-b439-b5c1a4414a02, 11be073c-ce91-44d8-833a-ef0b5e0b1672, 902bd279-e6c1-4a21-9637-a3a43f360bae, aa275fac-def5-42a2-8a61-660142acad8e, 83190b10-8060-47c1-8194-f6a5c4f0d703, 332ff0c7-0769-45ac-8f89-00aeff318467, f3c9e736-157c-4197-b49e-19e3674e4e0c
id: b80e29dc-3f25-4aa8-8fc8-fa5ea57cd051
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, findings=ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, moves=d904dc0d-231f-4e93-ac74-7babe888107d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, refusal=0942e445-0163-423e-8ba7-84dd8ac07045, statuses=209081f4-864f-4ef6-8909-65da87f135e5

Problem: statuses, findings and moves print in one shade, and the owner reads a disputed line, a refusal and a settled question alike.
Standard: a command colours its output when standard output is a terminal, prints plain when piped, and honours NO_COLOR.

- A) Yes.
  Why: the owner's eye finds the line to act on; an agent and the tests read piped output, plain.
  Costs: a palette per line kind, a terminal check before every print, and the page's own rendering of it.
- B) No.
  Why: one output everywhere; nothing to check.
  Costs: the owner reads every listing in one shade.

**Answer:** A

### What does the tool add to the shape of what it prints to a terminal?
depends: 1d7e59ea-99e3-4d4b-9928-5dabca62723d, f90dbd5b-8fbe-4899-8588-f8762795c2e2, 848e1b76-9cfe-483b-94dd-094eb9b2f995, 069a6b4a-de35-45f3-8c6c-8e254bf40565, 7857d633-2055-47eb-857f-cb70da9fa7ba, c84a4bd9-fc2d-44f0-b333-172978f61544, a6f41dee-2381-421a-bcc4-b14af243209b, b80e29dc-3f25-4aa8-8fc8-fa5ea57cd051=*
unrelated: 36dfbeaa-698d-4a57-b3b0-d425348e14a5, 2db33118-82b4-4edf-a9cb-e92ac62738b9, bf7da2dd-d8a7-41b8-9898-11cf68d0e833, a7b0b910-57f6-4c59-b904-b42f78db63f7, 08a1b488-32fc-4dc7-b4b3-93a6e768725f, bceceda8-133a-4c9b-b439-b5c1a4414a02, a82db5b7-969c-44e9-9eec-876ead7099d3, 3fc35d4f-90f3-4425-9dac-951a89631fdf, 4f925b13-ad16-48b8-acd1-3e30d6e308b6, 385a57d5-5f76-454a-ae5d-9b0d6f5c1492, 83190b10-8060-47c1-8194-f6a5c4f0d703, 332ff0c7-0769-45ac-8f89-00aeff318467, 05141f49-d9d2-407c-b60c-ebb195a05864
id: f3c9e736-157c-4197-b49e-19e3674e4e0c
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, findings=ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, moves=d904dc0d-231f-4e93-ac74-7babe888107d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, status=209081f4-864f-4ef6-8909-65da87f135e5, statuses=209081f4-864f-4ef6-8909-65da87f135e5

Problem: every line prints at one indent with no break between the questions, the findings and the moves, and a listing of a file runs on as one block.
Standard: a listing groups its lines, with a blank line between groups and a header per group, as git status does; piped output stays one line per item.

- A) Blank lines and headers between questions, findings and moves on a terminal; piped output one line per item.
  Why: the groups read apart, coloured or not; a line per item stays for the agent and the tests.
  Costs: a header per group, a terminal check before every listing, and the page's own rendering of the groups.
- B) Nothing.
  Why: one output on a terminal and in a pipe.
  Costs: the owner reads a file's listing as one block.
- C) Tables: columns aligned per group with a header row, on a terminal; piped output one line per item.
  Why: statuses and headings line up.
  Costs: column widths per group, wrapping of long headings, and a second form of every line.

**Answer:** A

### Does a command refuse what check lets pass?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, 0942e445-0163-423e-8ba7-84dd8ac07045, ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862
supersedes: fe14991d-9715-4ae0-9a23-d314a950d3b9, 069a6b4a-de35-45f3-8c6c-8e254bf40565, 184b5bcb-fa72-436b-92cb-4d4605366c59, de1c5329-e448-45eb-b14c-88b8654447a4, 7857d633-2055-47eb-857f-cb70da9fa7ba, b80e29dc-3f25-4aa8-8fc8-fa5ea57cd051, f3c9e736-157c-4197-b49e-19e3674e4e0c
unrelated: aac5c313-dedd-466b-a0c3-ead5e99ce913, f53ac3b9-efde-427c-8027-19988aa8fda7, 8081c82d-e5c6-485b-a302-07d586f1a01b, b0e865b4-9204-4c55-9f5b-dcb755b3a03b, 4f04bae4-98e3-48eb-9403-3257268e2d87, db07a54d-4d80-4035-a9ef-eabcc0c872b3, 3fc35d4f-90f3-4425-9dac-951a89631fdf, 902bd279-e6c1-4a21-9637-a3a43f360bae, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, 6108b8b2-911e-443b-abe0-4c6f8b407384, c3127af2-4c96-48d5-a164-bac49a9a09ec, aa475aa3-05b2-429a-b33a-46b1dcb52049, 05141f49-d9d2-407c-b60c-ebb195a05864, 44d02e7a-c902-4b89-973c-cf9d5c67c70e, 3cf56867-3918-4ce6-a296-2cc62a91b154, 343d0d28-2c5d-40fc-99aa-22bfba4ea79f, 8846f6d1-cd8c-4b87-b6f2-35bc8adf6763, 6875e4df-207a-4b62-8a89-d3179722a6d7
id: 80b89c24-7d65-43f4-bb03-55e9d85c327d
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ban=ef83beae-6857-4200-b251-a0fcc3c72f9d, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, move=d904dc0d-231f-4e93-ac74-7babe888107d, refusal=0942e445-0163-423e-8ba7-84dd8ac07045, refuse=0942e445-0163-423e-8ba7-84dd8ac07045, refused=0942e445-0163-423e-8ba7-84dd8ac07045, refuses=0942e445-0163-423e-8ba7-84dd8ac07045

Problem: approve refuses a draft with a refused word that check passes, and refuses a removed file that check and show report as awaiting approval; each command judges the records on its own.
Standard: one judgement per rule on the records, read by every command that acts on it.

- A) No.
  Why: every rule is judged once and read by every judging command; what approve refuses, check reports with the same move.
  Costs: check reads every rule those commands read; a rule on the input of one command, like ban, stays there.
- B) Yes.
  Why: each command judges what it needs when it writes.
  Costs: a draft passes check and the commit, then stops at approve, and the agent learns the rule from the refusal.

**Answer:** A

### What does a writing command print?
depends: e18b82d8-5271-4385-bbab-2b89e5da278f, ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, d904dc0d-231f-4e93-ac74-7babe888107d
supersedes: 069a6b4a-de35-45f3-8c6c-8e254bf40565, 184b5bcb-fa72-436b-92cb-4d4605366c59, 7857d633-2055-47eb-857f-cb70da9fa7ba, b80e29dc-3f25-4aa8-8fc8-fa5ea57cd051, f3c9e736-157c-4197-b49e-19e3674e4e0c, de1c5329-e448-45eb-b14c-88b8654447a4
unrelated: 36dfbeaa-698d-4a57-b3b0-d425348e14a5, aefa7f3c-70c3-412e-8d73-16ff9e2b703a, 3fc35d4f-90f3-4425-9dac-951a89631fdf, 848e1b76-9cfe-483b-94dd-094eb9b2f995, a6f41dee-2381-421a-bcc4-b14af243209b, c84a4bd9-fc2d-44f0-b333-172978f61544, dfd32ef6-c2ac-40ef-b8b3-9c58732516b1, 21ae8ae0-6884-4d3c-8464-024a8a5aae2d, 4f04bae4-98e3-48eb-9403-3257268e2d87, c3127af2-4c96-48d5-a164-bac49a9a09ec, 05141f49-d9d2-407c-b60c-ebb195a05864, 8f81dee1-9c19-447d-baa8-d7d0af691db8, 80b89c24-7d65-43f4-bb03-55e9d85c327d, 44d02e7a-c902-4b89-973c-cf9d5c67c70e, 3cf56867-3918-4ce6-a296-2cc62a91b154, 343d0d28-2c5d-40fc-99aa-22bfba4ea79f, 6875e4df-207a-4b62-8a89-d3179722a6d7
id: 8846f6d1-cd8c-4b87-b6f2-35bc8adf6763
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, answers=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, ban=ef83beae-6857-4200-b251-a0fcc3c72f9d, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, finding=ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, move=d904dc0d-231f-4e93-ac74-7babe888107d, moves=d904dc0d-231f-4e93-ac74-7babe888107d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c

Problem: approve, answer and ask print tier lines and the lines of other commands, install and the ban rebuild print lines no decision names, and what the owner reads after an act has no decision behind it.
Standard: a command that writes prints one line per event, as git commit does, or nothing, as mv does.

- A) The event it wrote, one line each, or the finding that stopped it, and nothing else.
  Why: the one who runs it reads what changed; the move after it comes from show.
  Costs: the move after an act takes a second command.
- B) Nothing on success; the finding that stopped it on a stop.
  Why: silence is success.
  Costs: an owner who runs approve sees no sign of what was approved.
- C) The event it wrote and the moves that follow it.
  Why: one command gives the agent its next move.
  Costs: ask repeats what show prints, and the owner's answer lines reach a terminal the owner answers from the page.

**Answer:** A

### What does a finding line name?
depends: ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, d904dc0d-231f-4e93-ac74-7babe888107d, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, 80b89c24-7d65-43f4-bb03-55e9d85c327d=*
supersedes: 184b5bcb-fa72-436b-92cb-4d4605366c59, de1c5329-e448-45eb-b14c-88b8654447a4
unrelated: a82db5b7-969c-44e9-9eec-876ead7099d3, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, 3fc35d4f-90f3-4425-9dac-951a89631fdf, 83190b10-8060-47c1-8194-f6a5c4f0d703, 069a6b4a-de35-45f3-8c6c-8e254bf40565, 1259323f-cdec-4101-9d47-a165a286089f, ad9761e2-6d64-4b45-b0ba-5c5d8d2b38bf, 4319a653-6901-446c-94ab-6420ef472134, c84a4bd9-fc2d-44f0-b333-172978f61544, 6108b8b2-911e-443b-abe0-4c6f8b407384, c3127af2-4c96-48d5-a164-bac49a9a09ec, 48f4647d-8c81-4e8a-a936-6b32f5fb0699, ee27fd4c-89b7-411f-a3e4-2535c49b6d1b, 44d02e7a-c902-4b89-973c-cf9d5c67c70e, 3cf56867-3918-4ce6-a296-2cc62a91b154, 8846f6d1-cd8c-4b87-b6f2-35bc8adf6763, 6875e4df-207a-4b62-8a89-d3179722a6d7
id: 343d0d28-2c5d-40fc-99aa-22bfba4ea79f
terms: answered=b36113a1-1384-4e4c-a798-8399f4181366, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, finding=ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, identifiers=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, move=d904dc0d-231f-4e93-ac74-7babe888107d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d

Problem: a finding names the number it measured and the act that lowers the number; the one who reads it clears the number, and the meaning the rule protects changes.
Standard: a compiler error names the rule a line breaks and leaves the fix to the writer.

- A) What the rule protects, in plain words, what it found, and the move as an act, not a diagnosis.
  Why: the one who reads it sees what the rule protects and rewrites; the tool never guesses the cause.
  Costs: the rule's words are the writer's, not the record's, and review alone holds them to it.
- B) The decision it judges by heading, what it found, and the move.
  Why: the record itself names the rule, in the words the owner answered.
  Costs: a terminal in any repository prints the headings and identifiers of this tool's own record.
- C) What it measured and the act that clears the measurement.
  Why: the shortest line, and the one the counter gives for free.
  Costs: the move names the counter, not the rule, and the fix that clears the counter can change the meaning.

**Answer:** A

### Does the tool colour its terminal output?
depends: 1d7e59ea-99e3-4d4b-9928-5dabca62723d, f90dbd5b-8fbe-4899-8588-f8762795c2e2, 343d0d28-2c5d-40fc-99aa-22bfba4ea79f=*
supersedes: b80e29dc-3f25-4aa8-8fc8-fa5ea57cd051, f3c9e736-157c-4197-b49e-19e3674e4e0c
unrelated: f53ac3b9-efde-427c-8027-19988aa8fda7, 36dfbeaa-698d-4a57-b3b0-d425348e14a5, 83190b10-8060-47c1-8194-f6a5c4f0d703, 8d3337b9-6fb3-4756-80f1-b388fe81d856, 2db33118-82b4-4edf-a9cb-e92ac62738b9, 7857d633-2055-47eb-857f-cb70da9fa7ba, 069a6b4a-de35-45f3-8c6c-8e254bf40565, c84a4bd9-fc2d-44f0-b333-172978f61544, 11be073c-ce91-44d8-833a-ef0b5e0b1672, 3fc35d4f-90f3-4425-9dac-951a89631fdf, 848e1b76-9cfe-483b-94dd-094eb9b2f995, aa275fac-def5-42a2-8a61-660142acad8e, 80b89c24-7d65-43f4-bb03-55e9d85c327d, 3cf56867-3918-4ce6-a296-2cc62a91b154, 8846f6d1-cd8c-4b87-b6f2-35bc8adf6763, 6875e4df-207a-4b62-8a89-d3179722a6d7
id: 44d02e7a-c902-4b89-973c-cf9d5c67c70e
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, findings=ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, moves=d904dc0d-231f-4e93-ac74-7babe888107d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, refusal=0942e445-0163-423e-8ba7-84dd8ac07045, statuses=209081f4-864f-4ef6-8909-65da87f135e5

Problem: statuses, findings and moves print in one shade, and the owner reads a disputed line, a refusal and a settled question alike.
Standard: a command colours its output when standard output is a terminal, prints plain when piped, and honours NO_COLOR.

- A) Yes.
  Why: the owner's eye finds the line to act on; an agent and the tests read piped output, plain.
  Costs: a palette per line kind, a terminal check before every print, and the page's own rendering of it.
- B) No.
  Why: one output everywhere; nothing to check.
  Costs: the owner reads every listing in one shade.

**Answer:** A

### What does the tool add to the shape of its terminal output?
depends: 1d7e59ea-99e3-4d4b-9928-5dabca62723d, f90dbd5b-8fbe-4899-8588-f8762795c2e2, 44d02e7a-c902-4b89-973c-cf9d5c67c70e=*
supersedes: f3c9e736-157c-4197-b49e-19e3674e4e0c
unrelated: b80e29dc-3f25-4aa8-8fc8-fa5ea57cd051, 385a57d5-5f76-454a-ae5d-9b0d6f5c1492, 7857d633-2055-47eb-857f-cb70da9fa7ba, 069a6b4a-de35-45f3-8c6c-8e254bf40565, a6f41dee-2381-421a-bcc4-b14af243209b, c84a4bd9-fc2d-44f0-b333-172978f61544, 08a1b488-32fc-4dc7-b4b3-93a6e768725f, bf7da2dd-d8a7-41b8-9898-11cf68d0e833, a7b0b910-57f6-4c59-b904-b42f78db63f7, a82db5b7-969c-44e9-9eec-876ead7099d3, 3fc35d4f-90f3-4425-9dac-951a89631fdf, 83190b10-8060-47c1-8194-f6a5c4f0d703, 8f81dee1-9c19-447d-baa8-d7d0af691db8, 80b89c24-7d65-43f4-bb03-55e9d85c327d, 3cf56867-3918-4ce6-a296-2cc62a91b154, 343d0d28-2c5d-40fc-99aa-22bfba4ea79f, 8846f6d1-cd8c-4b87-b6f2-35bc8adf6763
id: 6875e4df-207a-4b62-8a89-d3179722a6d7
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, findings=ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, moves=d904dc0d-231f-4e93-ac74-7babe888107d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, status=209081f4-864f-4ef6-8909-65da87f135e5, statuses=209081f4-864f-4ef6-8909-65da87f135e5

Problem: every line prints at one indent with no break between the questions, the findings and the moves, and a listing of a file runs on as one block.
Standard: a listing groups its lines, with a blank line between groups and a header per group, as git status does; piped output stays one line per item.

- A) Blank lines and headers between questions, findings and moves on a terminal; piped output one line per item.
  Why: the groups read apart, coloured or not; a line per item stays for the agent and the tests.
  Costs: a header per group, a terminal check before every listing, and the page's own rendering of the groups.
- B) Nothing.
  Why: one output on a terminal and in a pipe.
  Costs: the owner reads a file's listing as one block.
- C) Tables: columns aligned per group with a header row, on a terminal; piped output one line per item.
  Why: statuses and headings line up.
  Costs: column widths per group, wrapping of long headings, and a second form of every line.

**Answer:** A

### Does the ui show what the commands print?
depends: 7127ab1e-c313-4c51-9134-c09d551682cf, 1d7e59ea-99e3-4d4b-9928-5dabca62723d, 8846f6d1-cd8c-4b87-b6f2-35bc8adf6763=*, 6875e4df-207a-4b62-8a89-d3179722a6d7=*
supersedes: 7857d633-2055-47eb-857f-cb70da9fa7ba, b80e29dc-3f25-4aa8-8fc8-fa5ea57cd051, f3c9e736-157c-4197-b49e-19e3674e4e0c
unrelated: f53ac3b9-efde-427c-8027-19988aa8fda7, 36dfbeaa-698d-4a57-b3b0-d425348e14a5, 3fc35d4f-90f3-4425-9dac-951a89631fdf, 83190b10-8060-47c1-8194-f6a5c4f0d703, 848e1b76-9cfe-483b-94dd-094eb9b2f995, 069a6b4a-de35-45f3-8c6c-8e254bf40565, a6f41dee-2381-421a-bcc4-b14af243209b, c84a4bd9-fc2d-44f0-b333-172978f61544, 11be073c-ce91-44d8-833a-ef0b5e0b1672, 644b43ce-be10-4e9a-a7d5-71b27c51e365, 80b89c24-7d65-43f4-bb03-55e9d85c327d, 44d02e7a-c902-4b89-973c-cf9d5c67c70e, 343d0d28-2c5d-40fc-99aa-22bfba4ea79f
id: 3cf56867-3918-4ce6-a296-2cc62a91b154
terms: check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, findings=ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, moves=d904dc0d-231f-4e93-ac74-7babe888107d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, statuses=209081f4-864f-4ef6-8909-65da87f135e5, ui=7127ab1e-c313-4c51-9134-c09d551682cf

Problem: the ui copies check's whole output into a block, shows one open question at a time with no count of the rest, and its lines have no decision behind them apart from the owner's acts.
Standard: one view model, two renderings.

- A) Yes.
  Why: the page renders the same statuses, findings, moves and events the commands print, from the one computation.
  Costs: the page shows nothing the terminal cannot; a page form for every line kind.
- B) No.
  Why: the page is laid out for the owner alone, with its own lines.
  Costs: two sets of lines to hold right, and a line on the page with no decision behind it.

**Answer:** A

### What does a command print when it writes nothing?
depends: d904dc0d-231f-4e93-ac74-7babe888107d, ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, aac5c313-dedd-466b-a0c3-ead5e99ce913
supersedes: 848e1b76-9cfe-483b-94dd-094eb9b2f995, 36dfbeaa-698d-4a57-b3b0-d425348e14a5, 3fc35d4f-90f3-4425-9dac-951a89631fdf, bceceda8-133a-4c9b-b439-b5c1a4414a02, c84a4bd9-fc2d-44f0-b333-172978f61544, 8f81dee1-9c19-447d-baa8-d7d0af691db8, a6f41dee-2381-421a-bcc4-b14af243209b, 332ff0c7-0769-45ac-8f89-00aeff318467, 83190b10-8060-47c1-8194-f6a5c4f0d703
unrelated: f53ac3b9-efde-427c-8027-19988aa8fda7, 8081c82d-e5c6-485b-a302-07d586f1a01b, 902bd279-e6c1-4a21-9637-a3a43f360bae, 3cf56867-3918-4ce6-a296-2cc62a91b154, 8846f6d1-cd8c-4b87-b6f2-35bc8adf6763, 4f04bae4-98e3-48eb-9403-3257268e2d87, 34e81f57-9b3c-4dc4-a2ba-ca227523177d, 05141f49-d9d2-407c-b60c-ebb195a05864, d8a9dfff-de89-442c-a08c-8276efab3f36, cdc905a2-5b58-4c38-beff-4a8bd3b6f42e, bd83aa22-94fc-49ac-b215-a82990ff8c65, d1d6fed2-4e1a-44bd-b9cb-22f779a00bce, 91f9652a-f053-4b32-9534-c9c39e7639a4, b0acc95b-3dd5-4158-ac5c-65a2159972a9, 6e72f34e-d484-4c63-8332-d284857176fc, c5e10ae8-258d-4f29-b9fe-13cb7ad2a550
id: 1af76f91-5644-42ef-97ea-8f70f934c26c
terms: check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, findings=ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, move=d904dc0d-231f-4e93-ac74-7babe888107d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, status=209081f4-864f-4ef6-8909-65da87f135e5, statuses=209081f4-864f-4ef6-8909-65da87f135e5

Problem: show, check, plan and read each print lines the one who runs them has no move on, and the move hides among them.
Standard: a read-only command prints what it read and what follows from it, as git status does.

- A) What it reads from the records, its status, the findings, and the move that follows, and nothing else.
  Why: every line is read for a move; the record names each actor's inputs at each step.
  Costs: counts, pass notes and the hint line go; a passing check prints nothing.
- B) Everything it computed on the way: statuses, findings, counts, notes and the move.
  Why: the one who runs it sees that the command ran and what it covered.
  Costs: hundreds of lines around one move; each new command adds lines no decision names.

**Answer:** A

### Does check print a line with no move on it?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, d904dc0d-231f-4e93-ac74-7babe888107d, ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, 1af76f91-5644-42ef-97ea-8f70f934c26c=A
supersedes: 3fc35d4f-90f3-4425-9dac-951a89631fdf
unrelated: 36dfbeaa-698d-4a57-b3b0-d425348e14a5, 80b89c24-7d65-43f4-bb03-55e9d85c327d, 6108b8b2-911e-443b-abe0-4c6f8b407384, 83190b10-8060-47c1-8194-f6a5c4f0d703, 848e1b76-9cfe-483b-94dd-094eb9b2f995, 8846f6d1-cd8c-4b87-b6f2-35bc8adf6763, 05141f49-d9d2-407c-b60c-ebb195a05864, 3460d220-e523-448b-aeff-baa8051b7d06, c84a4bd9-fc2d-44f0-b333-172978f61544, 34e81f57-9b3c-4dc4-a2ba-ca227523177d, c10c3a49-e993-4c9f-a46b-4a1ba3646dcf, a469ecb9-a4f1-4a2a-89c5-7378cc1138b4, 2381d759-f85f-484e-9031-fadbe6ea045a, cdc905a2-5b58-4c38-beff-4a8bd3b6f42e, d1d6fed2-4e1a-44bd-b9cb-22f779a00bce, 91f9652a-f053-4b32-9534-c9c39e7639a4, b0acc95b-3dd5-4158-ac5c-65a2159972a9, 6e72f34e-d484-4c63-8332-d284857176fc, c5e10ae8-258d-4f29-b9fe-13cb7ad2a550
id: bd83aa22-94fc-49ac-b215-a82990ff8c65
terms: approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, blame file=108053a8-c2c3-4561-874c-2f573a5409f2, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, finding=ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, findings=ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, move=d904dc0d-231f-4e93-ac74-7babe888107d, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: on a pass check prints a tier line and a done line per topic file and counts of tested decisions and files awaiting approval, and none carries a move.
Standard: a checker prints its findings, each with the fix, and is silent on a pass.

- A) No.
  Why: every line of check is a finding with its move; a pass prints nothing, by hand or in a hook.
  Costs: hook flag, done, tier and tested lines go; no decision holds that a waiting blame file stops nothing.
- B) Yes.
  Why: the one who runs check sees what it covered.
  Costs: dozens of lines with no move on every run, and twice per commit without a flag.

**Answer:** A

### Does a command print a computed view under a flag?
depends: d904dc0d-231f-4e93-ac74-7babe888107d, daec9ffd-676f-4111-a359-2827bfd4527c, 1af76f91-5644-42ef-97ea-8f70f934c26c=A
supersedes: 36dfbeaa-698d-4a57-b3b0-d425348e14a5, 332ff0c7-0769-45ac-8f89-00aeff318467, a6f41dee-2381-421a-bcc4-b14af243209b, c84a4bd9-fc2d-44f0-b333-172978f61544, 8f81dee1-9c19-447d-baa8-d7d0af691db8, 83190b10-8060-47c1-8194-f6a5c4f0d703
unrelated: aac5c313-dedd-466b-a0c3-ead5e99ce913, f53ac3b9-efde-427c-8027-19988aa8fda7, bceceda8-133a-4c9b-b439-b5c1a4414a02, 44d02e7a-c902-4b89-973c-cf9d5c67c70e, 3cf56867-3918-4ce6-a296-2cc62a91b154, 8846f6d1-cd8c-4b87-b6f2-35bc8adf6763, 11be073c-ce91-44d8-833a-ef0b5e0b1672, 34e81f57-9b3c-4dc4-a2ba-ca227523177d, 848e1b76-9cfe-483b-94dd-094eb9b2f995, a469ecb9-a4f1-4a2a-89c5-7378cc1138b4, bd83aa22-94fc-49ac-b215-a82990ff8c65, d1d6fed2-4e1a-44bd-b9cb-22f779a00bce, 91f9652a-f053-4b32-9534-c9c39e7639a4, b0acc95b-3dd5-4158-ac5c-65a2159972a9, 6e72f34e-d484-4c63-8332-d284857176fc, c5e10ae8-258d-4f29-b9fe-13cb7ad2a550
id: cdc905a2-5b58-4c38-beff-4a8bd3b6f42e
terms: decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, finding=ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, move=d904dc0d-231f-4e93-ac74-7babe888107d, neighbours=daec9ffd-676f-4111-a359-2827bfd4527c, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, reader=be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, status=209081f4-864f-4ef6-8909-65da87f135e5

Problem: show prints the neighbours of every unasked question with the reader's word in its plain listing, five lines per heading that no move reads.
Standard: a computed view is printed on request, under a flag named for it, as git log --graph and ls -l do.

- A) Yes.
  Why: status and move in the listing; a computed view under a flag, or inside a finding or move needing it.
  Costs: a flag per view; a view with no flag is read from the files the tool writes.
- B) No.
  Why: a command prints the status and the move, and nothing computed, flag or not.
  Costs: the nearest decisions and the reader's word are read from plan's output or the reader's file alone.

**Answer:** A

### Does show print what a file holds past its latest approval?
depends: e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, 1d7e59ea-99e3-4d4b-9928-5dabca62723d, 08193e75-dd89-4857-8e9f-b19e1a9e75e6, 1af76f91-5644-42ef-97ea-8f70f934c26c=A
supersedes: bceceda8-133a-4c9b-b439-b5c1a4414a02, a6f41dee-2381-421a-bcc4-b14af243209b, 332ff0c7-0769-45ac-8f89-00aeff318467, 83190b10-8060-47c1-8194-f6a5c4f0d703, 8f81dee1-9c19-447d-baa8-d7d0af691db8
unrelated: 36dfbeaa-698d-4a57-b3b0-d425348e14a5, 1cdec741-98e1-4715-a7a5-94f7a73cd425, 912f4f3b-1213-435a-af0d-7617017fcff4, 90714223-52e3-4bb9-b0b4-2a8b87d74145, aa475aa3-05b2-429a-b33a-46b1dcb52049, 05141f49-d9d2-407c-b60c-ebb195a05864, c84a4bd9-fc2d-44f0-b333-172978f61544, 11be073c-ce91-44d8-833a-ef0b5e0b1672, 34e81f57-9b3c-4dc4-a2ba-ca227523177d, e879fe72-ec07-48cd-9902-2b158aaae726, 644b43ce-be10-4e9a-a7d5-71b27c51e365, cdc905a2-5b58-4c38-beff-4a8bd3b6f42e, bd83aa22-94fc-49ac-b215-a82990ff8c65, d1d6fed2-4e1a-44bd-b9cb-22f779a00bce, b0acc95b-3dd5-4158-ac5c-65a2159972a9, 6e72f34e-d484-4c63-8332-d284857176fc, c5e10ae8-258d-4f29-b9fe-13cb7ad2a550
id: 91f9652a-f053-4b32-9534-c9c39e7639a4
terms: approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, blame=108053a8-c2c3-4561-874c-2f573a5409f2, blame file=108053a8-c2c3-4561-874c-2f573a5409f2, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, refuses=0942e445-0163-423e-8ba7-84dd8ac07045, status=209081f4-864f-4ef6-8909-65da87f135e5, terms file=528fe6f6-6d77-42d5-9086-017048d1448d, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: the owner runs approve without a view of what it approves: show prints a topic file the same before and after approval, never its bodies, and refuses the terms file and a blame path.
Standard: a diff before a commit: the change is read before it is signed.

- A) Yes.
  Why: show takes every file approve takes and prints the headings, lines, bodies, entries or rows beyond its latest approval, marked.
  Costs: show parses the terms file and a blame file; a plan change prints the change, not the file.
- B) No.
  Why: show prints status, and a draft is read with git diff.
  Costs: the owner leaves the tool to see what approve will sign.

**Answer:** A

### What does show print when no file is named?
depends: 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, e2533580-1486-4dc9-8697-e6c53605fe72, 43778946-d44c-4acf-a766-0a4915fce199, d904dc0d-231f-4e93-ac74-7babe888107d, cdc905a2-5b58-4c38-beff-4a8bd3b6f42e=A
supersedes: c84a4bd9-fc2d-44f0-b333-172978f61544, 8f81dee1-9c19-447d-baa8-d7d0af691db8
unrelated: 36dfbeaa-698d-4a57-b3b0-d425348e14a5, 3fc35d4f-90f3-4425-9dac-951a89631fdf, 83190b10-8060-47c1-8194-f6a5c4f0d703, bceceda8-133a-4c9b-b439-b5c1a4414a02, 3cf56867-3918-4ce6-a296-2cc62a91b154, 848e1b76-9cfe-483b-94dd-094eb9b2f995, 8846f6d1-cd8c-4b87-b6f2-35bc8adf6763, a469ecb9-a4f1-4a2a-89c5-7378cc1138b4, a6f41dee-2381-421a-bcc4-b14af243209b, c3127af2-4c96-48d5-a164-bac49a9a09ec, bd83aa22-94fc-49ac-b215-a82990ff8c65, d1d6fed2-4e1a-44bd-b9cb-22f779a00bce, 91f9652a-f053-4b32-9534-c9c39e7639a4, 1af76f91-5644-42ef-97ea-8f70f934c26c, b0acc95b-3dd5-4158-ac5c-65a2159972a9, 6e72f34e-d484-4c63-8332-d284857176fc
id: c5e10ae8-258d-4f29-b9fe-13cb7ad2a550
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, move=d904dc0d-231f-4e93-ac74-7babe888107d, moves=d904dc0d-231f-4e93-ac74-7babe888107d, option=43778946-d44c-4acf-a766-0a4915fce199, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, status=209081f4-864f-4ef6-8909-65da87f135e5

Problem: an agent runs show to find its move and to read the decisions it builds on; a listing of everything runs to hundreds of lines, and a listing of moves alone leaves the decisions to a second form.
Standard: a listing command prints a summary and takes a flag or an argument for the whole.

- A) Every current decision as its heading and the option picked, and every move.
  Why: the agent reads the move first and the decisions it builds on second, in one run.
  Costs: a few lines per decision; the file form and the whole are a second run.
- B) Every move, and the decisions under a flag.
  Why: the shortest listing; the decisions come when asked for.
  Costs: the agent runs show twice before it builds.
- C) Every question of every file with its status and ties.
  Why: one listing holds everything.
  Costs: hundreds of lines to find one move, growing with every question.

**Answer:** A

### What does show print for a file under adrdom?
depends: d904dc0d-231f-4e93-ac74-7babe888107d, cdc905a2-5b58-4c38-beff-4a8bd3b6f42e=A, 91f9652a-f053-4b32-9534-c9c39e7639a4=A
supersedes: a6f41dee-2381-421a-bcc4-b14af243209b, 332ff0c7-0769-45ac-8f89-00aeff318467, 83190b10-8060-47c1-8194-f6a5c4f0d703, 8f81dee1-9c19-447d-baa8-d7d0af691db8
unrelated: 36dfbeaa-698d-4a57-b3b0-d425348e14a5, 4f925b13-ad16-48b8-acd1-3e30d6e308b6, bceceda8-133a-4c9b-b439-b5c1a4414a02, 16a71892-b348-4b9b-82c5-a9d375187bd3, 3cf56867-3918-4ce6-a296-2cc62a91b154, c84a4bd9-fc2d-44f0-b333-172978f61544, 08a1b488-32fc-4dc7-b4b3-93a6e768725f, e18b82d8-5271-4385-bbab-2b89e5da278f, 08193e75-dd89-4857-8e9f-b19e1a9e75e6, 3fc35d4f-90f3-4425-9dac-951a89631fdf, a469ecb9-a4f1-4a2a-89c5-7378cc1138b4, 05141f49-d9d2-407c-b60c-ebb195a05864, bd83aa22-94fc-49ac-b215-a82990ff8c65, d1d6fed2-4e1a-44bd-b9cb-22f779a00bce, 1af76f91-5644-42ef-97ea-8f70f934c26c, 6e72f34e-d484-4c63-8332-d284857176fc, c5e10ae8-258d-4f29-b9fe-13cb7ad2a550
id: b0acc95b-3dd5-4158-ac5c-65a2159972a9
terms: answers=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, move=d904dc0d-231f-4e93-ac74-7babe888107d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, status=209081f4-864f-4ef6-8909-65da87f135e5

Problem: show exists so that the one who runs it reads what their next move on the file needs, whatever kind of file it is.
Standard: a listing prints what the next move reads and takes a flag for the rest.

- A) What the next move on the file reads, and the rest under a flag.
  Why: a question with its status, the lines approve reads, an asked body; a new line or file kind changes nothing.
  Costs: which lines a move reads is derived from that move's decision, not listed in this one.
- B) Everything the file holds, with the status of each thing.
  Why: the whole file in one run, nothing under a flag.
  Costs: twenty unrelated names per question that no move reads, over half the output.
- C) Each thing's status only, and every line and body under a flag.
  Why: the shortest listing.
  Costs: the owner approves lines and answers bodies without seeing them.

**Answer:** A

### Does show print the reference lines of a question that is answered?
depends: aef1b384-2b33-47a2-92a8-ec7f8fb258ef, b36113a1-1384-4e4c-a798-8399f4181366, b0acc95b-3dd5-4158-ac5c-65a2159972a9=A
supersedes: 83190b10-8060-47c1-8194-f6a5c4f0d703
unrelated: 36dfbeaa-698d-4a57-b3b0-d425348e14a5, 946263aa-519a-47aa-b743-5cdcf52f5d35, b0554224-15e4-4e91-a2bb-9ae5667803f4, 11be073c-ce91-44d8-833a-ef0b5e0b1672, 6108b8b2-911e-443b-abe0-4c6f8b407384, bceceda8-133a-4c9b-b439-b5c1a4414a02, 3cf56867-3918-4ce6-a296-2cc62a91b154, 55a8abb2-0f6d-430b-99f9-a378f3f154e5, 332ff0c7-0769-45ac-8f89-00aeff318467, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, a6f41dee-2381-421a-bcc4-b14af243209b, cdc905a2-5b58-4c38-beff-4a8bd3b6f42e, bd83aa22-94fc-49ac-b215-a82990ff8c65, 91f9652a-f053-4b32-9534-c9c39e7639a4, 1af76f91-5644-42ef-97ea-8f70f934c26c, 6e72f34e-d484-4c63-8332-d284857176fc, c5e10ae8-258d-4f29-b9fe-13cb7ad2a550
id: d1d6fed2-4e1a-44bd-b9cb-22f779a00bce
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answered=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, blame lines=108053a8-c2c3-4561-874c-2f573a5409f2, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, move=d904dc0d-231f-4e93-ac74-7babe888107d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, supersedes=2bae531b-c4df-48a0-89a0-3d52726741ea

Problem: the depends, supersedes and blame lines of every answered question fill half of show's listing of a file, and no move on an answered question reads them.
Standard: a listing prints what the next step reads; what stands behind a closed item is shown on request.

- A) No.
  Why: the lines of an unanswered question are what approve reads; those of an answered one print under --dependents.
  Costs: an agent after an answered question's lines runs --dependents or reads the file.
- B) Yes.
  Why: every line of every question in one listing.
  Costs: lines no move reads make most of the listing of a settled file.

**Answer:** A

### What does show print of a reader's word on a question?
depends: be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, 277a8338-c57a-43cc-8b28-6b42e9347f6a, daec9ffd-676f-4111-a359-2827bfd4527c, b0acc95b-3dd5-4158-ac5c-65a2159972a9=A, cdc905a2-5b58-4c38-beff-4a8bd3b6f42e=A
supersedes: 332ff0c7-0769-45ac-8f89-00aeff318467
unrelated: 11be073c-ce91-44d8-833a-ef0b5e0b1672, 6108b8b2-911e-443b-abe0-4c6f8b407384, 83190b10-8060-47c1-8194-f6a5c4f0d703, bceceda8-133a-4c9b-b439-b5c1a4414a02, 2db33118-82b4-4edf-a9cb-e92ac62738b9, 3cf56867-3918-4ce6-a296-2cc62a91b154, 55a8abb2-0f6d-430b-99f9-a378f3f154e5, f5c242b6-e2f6-45a3-8c61-c743f3ad794f, a6f41dee-2381-421a-bcc4-b14af243209b, 21ae8ae0-6884-4d3c-8464-024a8a5aae2d, c3127af2-4c96-48d5-a164-bac49a9a09ec, 644b43ce-be10-4e9a-a7d5-71b27c51e365, 05141f49-d9d2-407c-b60c-ebb195a05864, 8f81dee1-9c19-447d-baa8-d7d0af691db8, bd83aa22-94fc-49ac-b215-a82990ff8c65, d1d6fed2-4e1a-44bd-b9cb-22f779a00bce, 91f9652a-f053-4b32-9534-c9c39e7639a4, 1af76f91-5644-42ef-97ea-8f70f934c26c, c5e10ae8-258d-4f29-b9fe-13cb7ad2a550
id: 6e72f34e-d484-4c63-8332-d284857176fc
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approving=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, move=d904dc0d-231f-4e93-ac74-7babe888107d, moves=d904dc0d-231f-4e93-ac74-7babe888107d, neighbour=daec9ffd-676f-4111-a359-2827bfd4527c, option=43778946-d44c-4acf-a766-0a4915fce199, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, reader=be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, supersedes=2bae531b-c4df-48a0-89a0-3d52726741ea

Problem: the owner approves a plan reading the agent's lines, and the reader's word on those lines, inconsistent, same or closes, is printed to the agent alone.
Standard: a review listing shows a disputed item beside the item, and the full analysis on request.

- A) Under the question, inconsistent, same or closes with its letter, beside a neighbour no supersedes line names; all under --reader.
  Why: the owner sees a disputed line and the option the record forces before approving; the rest stays under the flag.
  Costs: a view with no move on its line; show reads the reader's file for each question.
- B) Every word under --reader, nothing in the plain listing.
  Why: the plain listing stays the agent's lines and the moves.
  Costs: the owner runs the flag before each approval, or approves a disputed line unseen.
- C) Nothing: the reader's word is read in plan's output alone.
  Why: no new line form.
  Costs: the owner never sees the reader's word; the agent's line stands alone at approval.

**Answer:** A
