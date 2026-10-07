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
