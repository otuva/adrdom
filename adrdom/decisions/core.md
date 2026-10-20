# Core

What the tool holds fixed in every repository: what an answered question may no longer do, who
may run the owner's commands, what a question stands on, and the order questions reach the owner.

### Does a decision change after it is answered?
depends: 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, b36113a1-1384-4e4c-a798-8399f4181366, 2bae531b-c4df-48a0-89a0-3d52726741ea
id: dad5273c-afe2-4ab1-a451-e2a5d4d9a26d
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answered=b36113a1-1384-4e4c-a798-8399f4181366, answers=b36113a1-1384-4e4c-a798-8399f4181366, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, supersedes=2bae531b-c4df-48a0-89a0-3d52726741ea

Problem: when an answered decision can be edited, the spec moves under the owner and nobody can say what the owner picked when a line of code was written.
Standard: architecture decision records are immutable once accepted; a change is a new record that supersedes the old one.

- A) No: an answered decision never changes; it leaves force only when a newer decision supersedes it.
  Why: what the owner picked stays readable exactly as picked, and every change is a new question the owner answers.
  Costs: a decision that is slightly wrong is replaced whole by a new question, and its test moves to the successor.
- B) Yes, through separate owner commands to amend, drop or split a decision, each written to the ledger.
  Why: each kind of change has its own named route.
  Costs: three more owner commands to guard, and a decision's sentence changes under its own identifier.
- C) Yes: the owner edits the decision in place and the ledger records the edit.
  Why: the cheapest change for the owner.
  Costs: the text an agent built against moves without a new question, which is how a spec drifts.

**Answer:** A

### Does the tool guard against an agent that works around it on purpose?
depends: 1d7e59ea-99e3-4d4b-9928-5dabca62723d, f90dbd5b-8fbe-4899-8588-f8762795c2e2
id: 98d5d389-8315-41da-9970-666e31f3a94d
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, installed=1b98cdea-dc1b-4789-8f67-68d130d7a49d, installs=1b98cdea-dc1b-4789-8f67-68d130d7a49d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d

Problem: a gate looks strong or weak depending on the attacker assumed, and with no attacker named every owner-only mechanism is argued without a yardstick.
Standard: threat modelling names the adversary before choosing a control; controls against a careless insider and a hostile one are different systems.

- A) No: the tool guards against an agent that runs an owner command because it looks helpful in the moment, not one that works to get past the tool.
  Why: this is the failure every recorded case shows, and a guard against it fits inside one installed tool.
  Costs: an agent that sets out to pass as the owner gets through.
- B) Yes: the tool guards against an agent that actively works to get past it.
  Why: covers every agent.
  Costs: only a separate account or machine for the owner holds against that, which is a setup the owner builds, not a tool the owner installs.

**Answer:** A

### Does ask stop a question outside the plan?
depends: 3c8d4c9c-abca-4faa-b7c1-aae39951376c, 94677ed8-eaf6-46e2-bc71-7392fa0805bf, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd
id: da59b448-1afb-4469-b40e-69eda8f253c2
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: an agent with a topic file open writes every question it can think of in one pass, and the owner faces a pile of questions instead of a design.
Standard: spec-driven tools have the human approve a plan before tasks are written; most keep approval as a file state and do not stop work that ignores it.

- A) Yes: ask stops any question whose heading is not in the plan the owner approved.
  Why: a rule written only as text did not hold within a single session; a stop in the tool does.
  Costs: every topic file costs the owner an approve before any question is asked.
- B) No: the agent shows the plan to the owner in conversation, and nothing in the tool reads it.
  Why: nothing new in the tool.
  Costs: the agent is the only guard, and the agent is the party that writes the pile.
- C) No: the tool records the approved plan and check warns about questions outside it, stopping nothing.
  Why: the plan is on record without a wall.
  Costs: a warning an agent can walk past holds no better than B.

**Answer:** A

### Does check fail an answered question whose text changed?
depends: dad5273c-afe2-4ab1-a451-e2a5d4d9a26d=A, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, e18b82d8-5271-4385-bbab-2b89e5da278f
id: cd83e277-ebf0-4458-a77a-c526ab4cfeca
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answered=b36113a1-1384-4e4c-a798-8399f4181366, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, question=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: an answered decision never changes, but it sits in a text file an agent can edit; an edit nothing notices moves the decision silently.
Standard: tamper-evident logs store a hash of each record and verify it on every read.

- A) Yes: the ledger holds a hash of each answered question's text, and check fails any answered question whose text no longer matches it.
  Why: a silent edit becomes a failed check at the next commit.
  Costs: a harmless edit, such as a typo fix, fails the same way and needs a new question.
- B) No: git history shows every edit, and review catches it.
  Why: nothing new in the tool.
  Costs: the catch depends on someone reading the diff.

**Answer:** A

### What does an owner command look for before it runs?
depends: 98d5d389-8315-41da-9970-666e31f3a94d=*, 1d7e59ea-99e3-4d4b-9928-5dabca62723d, f90dbd5b-8fbe-4899-8588-f8762795c2e2
id: aa275fac-def5-42a2-8a61-660142acad8e
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d

Problem: an agent that runs an owner command because it seems helpful turns its own proposal into the owner's act; the command has to tell the two apart without a login.
Standard: allowlist over denylist; recognise the known principal instead of listing the threats.

- A) A sign of a person at the keyboard, an interactive terminal; without it the command stops, whatever the agent.
  Why: a new agent needs no update to the tool, and every agent stops by the same rule.
  Costs: an agent that opens a terminal on purpose passes, which the threat the tool guards against leaves out.
- B) Traces that known agent programs leave, such as the environment variables they set; the command stops when it finds one.
  Why: a recognised agent stops every time.
  Costs: an agent not on the list counts as the owner, and the list goes stale with every new agent.
- C) A secret the owner types on every owner command, such as a password.
  Why: holds even when an agent opens a terminal.
  Costs: the owner types it on every approve and answer, against a threat the tool does not guard against.

**Answer:** A

### Does check at a commit read the staged files?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, cd83e277-ebf0-4458-a77a-c526ab4cfeca
id: 34e81f57-9b3c-4dc4-a2ba-ca227523177d
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answered=b36113a1-1384-4e4c-a798-8399f4181366, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, question=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: a changed answered question can be staged while the working tree holds the old text, and check reads the working tree, so the commit passes.
Standard: pre-commit hooks judge the content the commit records, the way the pre-commit framework and lint-staged set unstaged changes aside first.

- A) Yes: at a commit, check reads every file it judges from the staged content, and the working tree only when it runs outside a commit.
  Why: what check passes is exactly what enters git.
  Costs: a fix left unstaged does not count until it is staged.
- B) No: check reads the working tree at a commit as it does outside one.
  Why: one way of reading, and the result matches what the agent sees on disk.
  Costs: a staged file that differs from the working tree enters git unchecked.

**Answer:** A

### What does check do with an untested current decision?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, e2533580-1486-4dc9-8697-e6c53605fe72, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f
supersedes: 33a45b9e-8150-488e-be70-7ab2ca9adaa6, 0b962bc8-05a8-48da-9fe8-bf77c7aa6e4a, b871e776-2da5-4f1e-b4c4-fb8229657e25, f2e31cbe-1ba2-4b77-910a-b9664ce0277f
id: 3460d220-e523-448b-aeff-baa8051b7d06
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, answered=b36113a1-1384-4e4c-a798-8399f4181366, answers=b36113a1-1384-4e4c-a798-8399f4181366, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c

Problem: a current decision with no test fails check once its tier is answered, so the last answer of a tier and every test of the tier reach git in one commit, and no answer of a later tier reaches git before the earlier tier is built.
Standard: a report separates a state from a violation; work in progress is counted and shown, and a gate fires only on a rule that is broken.

- A) Check reports it and fails nothing: an untested current decision is a state check prints on every run until a test names the decision.
  Why: an answer reaches git the moment it is given, and the build follows on any tier at any commit.
  Costs: nothing in the tool holds a decision to a test; a decision stays untested for as long as nothing is built on it.
- B) Check fails it: a current decision with no test fails check on every run.
  Why: every commit holds every current decision to a test.
  Costs: an answer cannot reach git before its test, so the answers and the build of a tier land in one commit.
- C) Check fails it once every question of its tier is answered, and reports it as waiting until then.
  Why: the answers of a tier reach git one by one, and a tier is built whole.
  Costs: the last answer of a tier and its build land in one commit, and no answer of a later tier reaches git before the earlier tier is built.

**Answer:** A

### What does a test carry to name its decision?
depends: 3460d220-e523-448b-aeff-baa8051b7d06=*, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, e877e4a9-2caf-4fe6-9f82-5476a2abb00b
id: 4319a653-6901-446c-94ab-6420ef472134
terms: answered=b36113a1-1384-4e4c-a798-8399f4181366, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b

Problem: the tool runs in repositories of any language and has to find which test names which decision without parsing that language.
Standard: cross-language tools mark a file with a comment carrying an identifier in a fixed form and find it by text search.

- A) A comment `adr: <id>` inside the test, found by text search in every tracked file outside adrdom.
  Why: one rule for any language, and a person can search for the identifier too.
  Costs: a comment can sit beside a test that asserts nothing; the tie names the test, not its honesty.
- B) A test file named by the decision's identifier, under a directory the tool owns.
  Why: the file being there is the tie, with no text to search.
  Costs: the repository's tests move into the tool's layout.
- C) A line in the decision naming the test by path and name.
  Why: the decision says where its test is.
  Costs: the decision changes whenever the test moves, and an answered decision cannot change.

**Answer:** A

### What does a depends entry name?
depends: aef1b384-2b33-47a2-92a8-ec7f8fb258ef, f590059d-b681-40ec-8a1c-c8916d71f6bb, 277a8338-c57a-43cc-8b28-6b42e9347f6a, 08193e75-dd89-4857-8e9f-b19e1a9e75e6, da59b448-1afb-4469-b40e-69eda8f253c2
supersedes: ad9761e2-6d64-4b45-b0ba-5c5d8d2b38bf, 1fb71321-bb84-492f-b9d5-1444c9c444af, 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f, 3b8da790-ea0c-4ca5-860a-980c7bc5f361, 672201dd-8f22-4dc3-8b21-a73eb7606e83, a82db5b7-969c-44e9-9eec-876ead7099d3, ee27fd4c-89b7-411f-a3e4-2535c49b6d1b, 7d47885f-3a45-45f9-a092-4314301f0d7b, eb2cde37-ebcb-4407-a5b0-9fcda4bd9a4e, 14180eab-60a4-48d3-8efc-c1b23a96ce3a, 3ce82f34-a4fc-4918-bb9b-65a0d81a6853, 7327cd58-ac58-40a4-8ef2-00072fb552b2, fe14991d-9715-4ae0-9a23-d314a950d3b9, 069a6b4a-de35-45f3-8c6c-8e254bf40565, 184b5bcb-fa72-436b-92cb-4d4605366c59, de1c5329-e448-45eb-b14c-88b8654447a4, 7857d633-2055-47eb-857f-cb70da9fa7ba, b80e29dc-3f25-4aa8-8fc8-fa5ea57cd051, f3c9e736-157c-4197-b49e-19e3674e4e0c
unrelated: 946263aa-519a-47aa-b743-5cdcf52f5d35, 4319a653-6901-446c-94ab-6420ef472134, 2bae531b-c4df-48a0-89a0-3d52726741ea, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, c6da863d-5085-402f-815a-9bb832b4df7e, c3127af2-4c96-48d5-a164-bac49a9a09ec, 08a1b488-32fc-4dc7-b4b3-93a6e768725f, 7c0b9fe3-6038-4a23-93fe-88a31c7eaf40, 49a1c3a8-07ea-448a-8926-2362c0617758, da80931c-68cf-4c51-acce-060307bbd973, 9bf76db8-4821-4257-ac49-4c22c2a0e538, 86d5e944-ecfa-44bc-9819-83ee4b96890d, f1b4c4cc-97bd-446f-87b4-df5f1b6f4da2, 010901e0-c02d-49eb-98bc-d584756d7f51, 0e38f001-0226-4927-9a78-2e9fc1ec18d4, 1d7c758d-1fd5-4e0c-8559-517315b15aaa
id: 9afe335c-2a7a-4625-b115-f74d3f091cac
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, checks=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, options=43778946-d44c-4acf-a766-0a4915fce199, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, reader=be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, supersede=2bae531b-c4df-48a0-89a0-3d52726741ea, term=f590059d-b681-40ec-8a1c-c8916d71f6bb, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: a question that assumes another question's answer carries that assumption nowhere the tool or the owner can see; it surfaces only when the owner reads the options.
Standard: dependency-ordered task lists carry a prerequisite field per item, and an item names items of its own list.

- A) A term, or a question of the same topic file.
  Why: the assumption sits beside the heading where the owner reads it, and the order it gives stays inside one file.
  Costs: a question standing on a decision of another file names it nowhere; only the reader ties that pair.
- B) A term, a question of the same topic file, or a current decision of another file.
  Why: every assumption has a name, wherever its decision lives.
  Costs: a supersede in one file reaches every file a line points at.
- C) Free text the owner reads and the tool never resolves.
  Why: the agent states the assumption in its own words.
  Costs: the tool checks nothing, and no order can be derived from it.

**Answer:** A

### What stops a question that has no depends line?
depends: aef1b384-2b33-47a2-92a8-ec7f8fb258ef, 277a8338-c57a-43cc-8b28-6b42e9347f6a, 9afe335c-2a7a-4625-b115-f74d3f091cac=A|B
supersedes: ad9761e2-6d64-4b45-b0ba-5c5d8d2b38bf, 1fb71321-bb84-492f-b9d5-1444c9c444af, 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f, 3b8da790-ea0c-4ca5-860a-980c7bc5f361, 672201dd-8f22-4dc3-8b21-a73eb7606e83, a82db5b7-969c-44e9-9eec-876ead7099d3, ee27fd4c-89b7-411f-a3e4-2535c49b6d1b, 7d47885f-3a45-45f9-a092-4314301f0d7b, eb2cde37-ebcb-4407-a5b0-9fcda4bd9a4e, 14180eab-60a4-48d3-8efc-c1b23a96ce3a, 3ce82f34-a4fc-4918-bb9b-65a0d81a6853, 7327cd58-ac58-40a4-8ef2-00072fb552b2, fe14991d-9715-4ae0-9a23-d314a950d3b9, 069a6b4a-de35-45f3-8c6c-8e254bf40565, 184b5bcb-fa72-436b-92cb-4d4605366c59, de1c5329-e448-45eb-b14c-88b8654447a4, 7857d633-2055-47eb-857f-cb70da9fa7ba, b80e29dc-3f25-4aa8-8fc8-fa5ea57cd051, f3c9e736-157c-4197-b49e-19e3674e4e0c
unrelated: 946263aa-519a-47aa-b743-5cdcf52f5d35, f571a454-3e6c-46c1-b754-0822a5d1d667, 83190b10-8060-47c1-8194-f6a5c4f0d703, 62e50382-db1b-4649-9f26-7f44c379d6fe, c6e0b7d6-cdec-46da-b2bc-cda218673868, 0a742360-83c2-479d-b858-4bea646c9b36, 506b3c6d-3d0a-4a02-98de-3b8a573d2fe9, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, 711a7636-9f4d-402d-ae62-265208adf0ac, e3200081-9995-4bbe-983f-2f430dc341e7, b782664f-7465-4e1e-99e0-13f6dd2de774, 7c0b9fe3-6038-4a23-93fe-88a31c7eaf40, 49a1c3a8-07ea-448a-8926-2362c0617758, da80931c-68cf-4c51-acce-060307bbd973, 86d5e944-ecfa-44bc-9819-83ee4b96890d, f1b4c4cc-97bd-446f-87b4-df5f1b6f4da2, 010901e0-c02d-49eb-98bc-d584756d7f51, 0e38f001-0226-4927-9a78-2e9fc1ec18d4, 1d7c758d-1fd5-4e0c-8559-517315b15aaa, 462e22c6-8b23-4061-9a2a-bbf8691fce60, d808917b-0717-45ad-ab83-9fa274b9a9d4, da59b448-1afb-4469-b40e-69eda8f253c2, 3fc35d4f-90f3-4425-9dac-951a89631fdf, 6b3d8e59-e145-49e1-860d-a79d753d745c, c3127af2-4c96-48d5-a164-bac49a9a09ec
id: 9bf76db8-4821-4257-ac49-4c22c2a0e538
terms: answered=b36113a1-1384-4e4c-a798-8399f4181366, answers=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, finding=ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, refused=0942e445-0163-423e-8ba7-84dd8ac07045, stands on=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, terms=f590059d-b681-40ec-8a1c-c8916d71f6bb

Problem: a question with no line and a question whose assumption was forgotten look the same.
Standard: a required field: an item with the field empty is refused, not read as none.

- A) Approve and ask: the plan is refused until every question carries the line.
  Why: a forgotten assumption cannot pass as none.
  Costs: a question standing on terms alone still writes the terms it stands on.
- B) Nothing: a question with no line stands on nothing.
  Why: a question that stands on nothing costs no line.
  Costs: a missing line and a forgotten assumption look the same.
- C) Check: a finding on the question, and the plan goes on.
  Why: the owner approves and answers with the gap in view.
  Costs: a question is answered on an assumption nobody wrote.

**Answer:** A

### What stops a depends entry that resolves to nothing?
depends: aef1b384-2b33-47a2-92a8-ec7f8fb258ef, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, 3c8d4c9c-abca-4faa-b7c1-aae39951376c, 9afe335c-2a7a-4625-b115-f74d3f091cac=A|B, 7c0b9fe3-6038-4a23-93fe-88a31c7eaf40=*
supersedes: ad9761e2-6d64-4b45-b0ba-5c5d8d2b38bf, 1fb71321-bb84-492f-b9d5-1444c9c444af, 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f, 3b8da790-ea0c-4ca5-860a-980c7bc5f361, 672201dd-8f22-4dc3-8b21-a73eb7606e83, a82db5b7-969c-44e9-9eec-876ead7099d3, ee27fd4c-89b7-411f-a3e4-2535c49b6d1b, 7d47885f-3a45-45f9-a092-4314301f0d7b, eb2cde37-ebcb-4407-a5b0-9fcda4bd9a4e, 14180eab-60a4-48d3-8efc-c1b23a96ce3a, 3ce82f34-a4fc-4918-bb9b-65a0d81a6853, 7327cd58-ac58-40a4-8ef2-00072fb552b2, fe14991d-9715-4ae0-9a23-d314a950d3b9, 069a6b4a-de35-45f3-8c6c-8e254bf40565, 184b5bcb-fa72-436b-92cb-4d4605366c59, de1c5329-e448-45eb-b14c-88b8654447a4, 7857d633-2055-47eb-857f-cb70da9fa7ba, b80e29dc-3f25-4aa8-8fc8-fa5ea57cd051, f3c9e736-157c-4197-b49e-19e3674e4e0c
unrelated: 946263aa-519a-47aa-b743-5cdcf52f5d35, d3be71e4-ce7b-43b0-86a8-e86868521f99, 506b3c6d-3d0a-4a02-98de-3b8a573d2fe9, da59b448-1afb-4469-b40e-69eda8f253c2, db07a54d-4d80-4035-a9ef-eabcc0c872b3, 3dba486c-a399-4ce3-a794-e38c4f733403, c3127af2-4c96-48d5-a164-bac49a9a09ec, c96518bf-470d-45c0-8b86-eaf8f54cb490, 0487a1cf-eb61-4bf8-bd16-eed899d22d2c, 0c3d498a-c540-47f3-8384-050fc044c109, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, 711a7636-9f4d-402d-ae62-265208adf0ac, 4f04bae4-98e3-48eb-9403-3257268e2d87, 3fc35d4f-90f3-4425-9dac-951a89631fdf, 48f4647d-8c81-4e8a-a936-6b32f5fb0699, aa475aa3-05b2-429a-b33a-46b1dcb52049, da80931c-68cf-4c51-acce-060307bbd973, 9bf76db8-4821-4257-ac49-4c22c2a0e538, 86d5e944-ecfa-44bc-9819-83ee4b96890d, f1b4c4cc-97bd-446f-87b4-df5f1b6f4da2, 010901e0-c02d-49eb-98bc-d584756d7f51, 0e38f001-0226-4927-9a78-2e9fc1ec18d4, 1d7c758d-1fd5-4e0c-8559-517315b15aaa, 8081c82d-e5c6-485b-a302-07d586f1a01b, bf6697e3-8832-4b90-b036-8fecedb55406, 912f4f3b-1213-435a-af0d-7617017fcff4, 05141f49-d9d2-407c-b60c-ebb195a05864, 7cf81ded-be48-4777-b83e-f8a2998c1ab5
id: 49a1c3a8-07ea-448a-8926-2362c0617758
terms: approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, checked=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, finding=ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, moves=d904dc0d-231f-4e93-ac74-7babe888107d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, refused=0942e445-0163-423e-8ba7-84dd8ac07045, term=f590059d-b681-40ec-8a1c-c8916d71f6bb

Problem: an entry that names nothing the records hold gives the tool nothing to order by and the owner nothing to read.
Standard: a prerequisite that does not resolve stops the item, as a build stops on a missing input.

- A) Approve and ask: the plan is refused and the entry named.
  Why: every line in the record points at something.
  Costs: a renamed term or heading stops the plan until the entry is rewritten.
- B) Nothing: the entry stays as text.
  Why: a bad entry costs no stop.
  Costs: a line nothing resolves orders nothing and reads as an assumption that was checked.
- C) Check: a finding on the entry, and the plan goes on.
  Why: the plan moves while the entry is fixed.
  Costs: the owner approves a line that points nowhere.

**Answer:** A

### What bounds the questions a depends line names?
depends: aef1b384-2b33-47a2-92a8-ec7f8fb258ef, 277a8338-c57a-43cc-8b28-6b42e9347f6a, 9afe335c-2a7a-4625-b115-f74d3f091cac=A
unrelated: 946263aa-519a-47aa-b743-5cdcf52f5d35, a82db5b7-969c-44e9-9eec-876ead7099d3, d808917b-0717-45ad-ab83-9fa274b9a9d4, 83190b10-8060-47c1-8194-f6a5c4f0d703, 0487a1cf-eb61-4bf8-bd16-eed899d22d2c, 55a8abb2-0f6d-430b-99f9-a378f3f154e5, ad9761e2-6d64-4b45-b0ba-5c5d8d2b38bf, 1fb71321-bb84-492f-b9d5-1444c9c444af, 0a742360-83c2-479d-b858-4bea646c9b36, b0e865b4-9204-4c55-9f5b-dcb755b3a03b, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, da59b448-1afb-4469-b40e-69eda8f253c2, c3127af2-4c96-48d5-a164-bac49a9a09ec, eb2cde37-ebcb-4407-a5b0-9fcda4bd9a4e, 7c0b9fe3-6038-4a23-93fe-88a31c7eaf40, 49a1c3a8-07ea-448a-8926-2362c0617758, da80931c-68cf-4c51-acce-060307bbd973, 9bf76db8-4821-4257-ac49-4c22c2a0e538, 86d5e944-ecfa-44bc-9819-83ee4b96890d, f1b4c4cc-97bd-446f-87b4-df5f1b6f4da2, 0e38f001-0226-4927-9a78-2e9fc1ec18d4, 1d7c758d-1fd5-4e0c-8559-517315b15aaa
id: 010901e0-c02d-49eb-98bc-d584756d7f51
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, moves=d904dc0d-231f-4e93-ac74-7babe888107d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, reader=be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, stands on=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, supersede=2bae531b-c4df-48a0-89a0-3d52726741ea

Problem: nothing bounds what a line names; a line naming every nearby question makes the question stand on everything, and one supersede reaches the whole file.
Standard: a dependency graph keeps its fan-in small, a few prerequisites per item, so a reader can follow it.

- A) A cap of two set in the tool; approve and ask stop a plan with a line over it.
  Why: the agent names what the question stands on and nothing nearby, and a supersede reaches few questions.
  Costs: a question standing on three others splits or drops one, and the dropped one is named nowhere.
- B) Nothing: a line names as many questions as the agent writes.
  Why: nothing new in the tool.
  Costs: a line that names every nearby question passes, and a supersede reaches the whole file.
- C) A cap of two the owner moves with an owner command the ledger records.
  Why: the value moves without a question or a release.
  Costs: one more owner command and ledger event to guard, for a value no plan has pushed against.

**Answer:** A

### What does a reference in a topic file carry?
depends: e877e4a9-2caf-4fe6-9f82-5476a2abb00b, 08193e75-dd89-4857-8e9f-b19e1a9e75e6, 9afe335c-2a7a-4625-b115-f74d3f091cac=A|B
supersedes: 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f, 3b8da790-ea0c-4ca5-860a-980c7bc5f361, 672201dd-8f22-4dc3-8b21-a73eb7606e83, a82db5b7-969c-44e9-9eec-876ead7099d3, ee27fd4c-89b7-411f-a3e4-2535c49b6d1b
unrelated: 946263aa-519a-47aa-b743-5cdcf52f5d35, de1c5329-e448-45eb-b14c-88b8654447a4, 7a1e7b81-0967-4ed2-8a8a-c7153056836f, 23f21671-e34c-44aa-8804-6163d975807f, 83190b10-8060-47c1-8194-f6a5c4f0d703, b0e865b4-9204-4c55-9f5b-dcb755b3a03b, 6b3d8e59-e145-49e1-860d-a79d753d745c, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, c6da863d-5085-402f-815a-9bb832b4df7e, 6108b8b2-911e-443b-abe0-4c6f8b407384, 7c306235-6243-4d79-b9aa-08ed19662779, 49a1c3a8-07ea-448a-8926-2362c0617758, da80931c-68cf-4c51-acce-060307bbd973, 9bf76db8-4821-4257-ac49-4c22c2a0e538, 86d5e944-ecfa-44bc-9819-83ee4b96890d, f1b4c4cc-97bd-446f-87b4-df5f1b6f4da2, 010901e0-c02d-49eb-98bc-d584756d7f51, 0e38f001-0226-4927-9a78-2e9fc1ec18d4, 1d7c758d-1fd5-4e0c-8559-517315b15aaa, 79da6500-2246-4a80-baf1-0eaf7cad49ee, aef1b384-2b33-47a2-92a8-ec7f8fb258ef, 94677ed8-eaf6-46e2-bc71-7392fa0805bf, 292a10b8-c287-4d71-b2bd-840da0a6de61, adee66f2-c8ff-477f-8ac4-a497f694c912, 332ff0c7-0769-45ac-8f89-00aeff318467
id: 7c0b9fe3-6038-4a23-93fe-88a31c7eaf40
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, term=f590059d-b681-40ec-8a1c-c8916d71f6bb, terms=f590059d-b681-40ec-8a1c-c8916d71f6bb, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: when a heading is reworded or a term renamed, every reference that names it by its text breaks, and each one is fixed by hand.
Standard: reference by a stable identifier; the name is only shown to people.

- A) The identifier of its target: the agent writes the name, approve turns it, show and the page turn it back.
  Why: a rename breaks no reference, and terms, decisions and questions follow one rule.
  Costs: the raw file does not say what a reference points at without show or the page.
- B) The text of its target; a rename breaks it and the agent fixes it by hand.
  Why: the raw file reads on its own.
  Costs: every rename is an edit on every line that points at the target.

**Answer:** A

### What are the tiers derived from?
depends: f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, 9afe335c-2a7a-4625-b115-f74d3f091cac=A|B, 9bf76db8-4821-4257-ac49-4c22c2a0e538=A
supersedes: 1fb71321-bb84-492f-b9d5-1444c9c444af, 7d47885f-3a45-45f9-a092-4314301f0d7b, eb2cde37-ebcb-4407-a5b0-9fcda4bd9a4e, 14180eab-60a4-48d3-8efc-c1b23a96ce3a, 3ce82f34-a4fc-4918-bb9b-65a0d81a6853, 7327cd58-ac58-40a4-8ef2-00072fb552b2, fe14991d-9715-4ae0-9a23-d314a950d3b9, 069a6b4a-de35-45f3-8c6c-8e254bf40565, 184b5bcb-fa72-436b-92cb-4d4605366c59, de1c5329-e448-45eb-b14c-88b8654447a4, 7857d633-2055-47eb-857f-cb70da9fa7ba, b80e29dc-3f25-4aa8-8fc8-fa5ea57cd051, f3c9e736-157c-4197-b49e-19e3674e4e0c
unrelated: 506b3c6d-3d0a-4a02-98de-3b8a573d2fe9, 99619e27-19b6-47ef-83b9-c79ac30ee2f0, f8d92841-aef7-4cd7-bd94-0e07c03e4cb6, 672201dd-8f22-4dc3-8b21-a73eb7606e83, 108053a8-c2c3-4561-874c-2f573a5409f2, 1d7e59ea-99e3-4d4b-9928-5dabca62723d, 2bae531b-c4df-48a0-89a0-3d52726741ea, 946263aa-519a-47aa-b743-5cdcf52f5d35, c3127af2-4c96-48d5-a164-bac49a9a09ec, 385a57d5-5f76-454a-ae5d-9b0d6f5c1492, ad9761e2-6d64-4b45-b0ba-5c5d8d2b38bf, 7c0b9fe3-6038-4a23-93fe-88a31c7eaf40, 49a1c3a8-07ea-448a-8926-2362c0617758, da80931c-68cf-4c51-acce-060307bbd973, 86d5e944-ecfa-44bc-9819-83ee4b96890d, 010901e0-c02d-49eb-98bc-d584756d7f51, 0e38f001-0226-4927-9a78-2e9fc1ec18d4, 1d7c758d-1fd5-4e0c-8559-517315b15aaa
id: f1b4c4cc-97bd-446f-87b4-df5f1b6f4da2
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answers=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, checked=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, stands on=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, tiers=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c

Problem: questions that assume each other's answers reach the owner side by side when nothing in the file orders them.
Standard: build systems derive execution order from declared dependencies and stop on a cycle; nobody writes the order by hand.

- A) The depends lines alone: a question is one tier above the highest question of its file it stands on.
  Why: nothing in the file states an order, so a question cannot sit beside one it stands on.
  Costs: the raw file shows no order without show, and a cycle in the lines stops approve.
- B) Tier headings the agent writes, checked against the depends lines.
  Why: the raw file shows the order.
  Costs: two sources for one order, and every mismatch is a failure the agent fixes by hand.

**Answer:** A

### What does ask put to the owner at one run?
depends: 3c8d4c9c-abca-4faa-b7c1-aae39951376c, 1d7e59ea-99e3-4d4b-9928-5dabca62723d, 94677ed8-eaf6-46e2-bc71-7392fa0805bf, f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, f1b4c4cc-97bd-446f-87b4-df5f1b6f4da2=*
supersedes: 7d47885f-3a45-45f9-a092-4314301f0d7b, 3ce82f34-a4fc-4918-bb9b-65a0d81a6853, 7327cd58-ac58-40a4-8ef2-00072fb552b2, eb2cde37-ebcb-4407-a5b0-9fcda4bd9a4e, 14180eab-60a4-48d3-8efc-c1b23a96ce3a, fe14991d-9715-4ae0-9a23-d314a950d3b9, 069a6b4a-de35-45f3-8c6c-8e254bf40565, 184b5bcb-fa72-436b-92cb-4d4605366c59, de1c5329-e448-45eb-b14c-88b8654447a4, 7857d633-2055-47eb-857f-cb70da9fa7ba, b80e29dc-3f25-4aa8-8fc8-fa5ea57cd051, f3c9e736-157c-4197-b49e-19e3674e4e0c
unrelated: da59b448-1afb-4469-b40e-69eda8f253c2, b782664f-7465-4e1e-99e0-13f6dd2de774, ef7ec04d-9cdd-40a1-8246-42a4a0200828, b36113a1-1384-4e4c-a798-8399f4181366, 43778946-d44c-4acf-a766-0a4915fce199, 3afef03f-9017-4638-9759-e010d44d536b, 506b3c6d-3d0a-4a02-98de-3b8a573d2fe9, 6b3d8e59-e145-49e1-860d-a79d753d745c, f8d92841-aef7-4cd7-bd94-0e07c03e4cb6, c3127af2-4c96-48d5-a164-bac49a9a09ec, 1fb71321-bb84-492f-b9d5-1444c9c444af, 7c0b9fe3-6038-4a23-93fe-88a31c7eaf40, 49a1c3a8-07ea-448a-8926-2362c0617758, da80931c-68cf-4c51-acce-060307bbd973, 9bf76db8-4821-4257-ac49-4c22c2a0e538, 86d5e944-ecfa-44bc-9819-83ee4b96890d, 010901e0-c02d-49eb-98bc-d584756d7f51, 0e38f001-0226-4927-9a78-2e9fc1ec18d4, 9afe335c-2a7a-4625-b115-f74d3f091cac
id: 1d7c758d-1fd5-4e0c-8559-517315b15aaa
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, answers=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, stands on=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, tiers=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c

Problem: a plan of four tiers reaches the owner in four runs, each waiting on a commit and an answer, though the owner read the plan whole at approve.
Standard: a code review puts the whole change before the reviewer at once, in the order the files stand.

- A) Every unanswered question of the file at once, whatever its tier.
  Why: one run, one commit and one reading put the whole plan before the owner.
  Costs: a higher question is asked before the answer it stands on, and a lower answer may remove it.
- B) The lowest tier of the file holding an unanswered question.
  Why: every question reaches the owner after the answers it stands on, so none is asked in vain.
  Costs: a plan of four tiers takes four runs, each waiting on a commit and the owner's answers.

**Answer:** A

### Does answer take a question standing on a question that has no decision?
depends: b36113a1-1384-4e4c-a798-8399f4181366, 9d634996-475e-4ddc-8113-05c404ad9d5c, 1d7c758d-1fd5-4e0c-8559-517315b15aaa=A
supersedes: 7327cd58-ac58-40a4-8ef2-00072fb552b2
unrelated: 946263aa-519a-47aa-b743-5cdcf52f5d35, 506b3c6d-3d0a-4a02-98de-3b8a573d2fe9, 11be073c-ce91-44d8-833a-ef0b5e0b1672, 462e22c6-8b23-4061-9a2a-bbf8691fce60, e3200081-9995-4bbe-983f-2f430dc341e7, 83190b10-8060-47c1-8194-f6a5c4f0d703, b782664f-7465-4e1e-99e0-13f6dd2de774, 55a8abb2-0f6d-430b-99f9-a378f3f154e5, dad5273c-afe2-4ab1-a451-e2a5d4d9a26d, 6b3d8e59-e145-49e1-860d-a79d753d745c, f8d92841-aef7-4cd7-bd94-0e07c03e4cb6, 7c0b9fe3-6038-4a23-93fe-88a31c7eaf40, 49a1c3a8-07ea-448a-8926-2362c0617758, 9bf76db8-4821-4257-ac49-4c22c2a0e538, 86d5e944-ecfa-44bc-9819-83ee4b96890d, f1b4c4cc-97bd-446f-87b4-df5f1b6f4da2, 010901e0-c02d-49eb-98bc-d584756d7f51, 0e38f001-0226-4927-9a78-2e9fc1ec18d4, 9afe335c-2a7a-4625-b115-f74d3f091cac, 9ba58b6a-bc85-44a5-a720-d8e7a6ff7b31, 14180eab-60a4-48d3-8efc-c1b23a96ce3a, c3127af2-4c96-48d5-a164-bac49a9a09ec
id: da80931c-68cf-4c51-acce-060307bbd973
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, answer set=9d634996-475e-4ddc-8113-05c404ad9d5c, answered=b36113a1-1384-4e4c-a798-8399f4181366, answers=b36113a1-1384-4e4c-a798-8399f4181366, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, refuses=0942e445-0163-423e-8ba7-84dd8ac07045, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c

Problem: with the whole file asked at once, a higher question answered first may be removed by the lower answer, and the owner's answer is lost.
Standard: a dependency is resolved before what rests on it; a build never links against an object not yet built.

- A) No.
  Why: an answer is given only when every answer set it rests on is settled, so none is thrown away.
  Costs: answer refuses out of order, so the owner answers a file from its first tier up.
- B) Yes.
  Why: the owner answers in any order.
  Costs: a lower answer may remove an answered question, against the rule that an answered decision never changes.

**Answer:** A

### What bounds the questions at one tier?
depends: f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, f1b4c4cc-97bd-446f-87b4-df5f1b6f4da2=*
supersedes: eb2cde37-ebcb-4407-a5b0-9fcda4bd9a4e, 14180eab-60a4-48d3-8efc-c1b23a96ce3a
unrelated: 946263aa-519a-47aa-b743-5cdcf52f5d35, 506b3c6d-3d0a-4a02-98de-3b8a573d2fe9, d808917b-0717-45ad-ab83-9fa274b9a9d4, f8d92841-aef7-4cd7-bd94-0e07c03e4cb6, 7d47885f-3a45-45f9-a092-4314301f0d7b, 1fb71321-bb84-492f-b9d5-1444c9c444af, 277a8338-c57a-43cc-8b28-6b42e9347f6a, 292a10b8-c287-4d71-b2bd-840da0a6de61, 7327cd58-ac58-40a4-8ef2-00072fb552b2, da59b448-1afb-4469-b40e-69eda8f253c2, e3200081-9995-4bbe-983f-2f430dc341e7, 7c0b9fe3-6038-4a23-93fe-88a31c7eaf40, 49a1c3a8-07ea-448a-8926-2362c0617758, da80931c-68cf-4c51-acce-060307bbd973, 9bf76db8-4821-4257-ac49-4c22c2a0e538, 86d5e944-ecfa-44bc-9819-83ee4b96890d, 010901e0-c02d-49eb-98bc-d584756d7f51, 9afe335c-2a7a-4625-b115-f74d3f091cac, 1d7c758d-1fd5-4e0c-8559-517315b15aaa
id: 0e38f001-0226-4927-9a78-2e9fc1ec18d4
terms: approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, moves=d904dc0d-231f-4e93-ac74-7babe888107d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, terms=f590059d-b681-40ec-8a1c-c8916d71f6bb, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, topic=292a10b8-c287-4d71-b2bd-840da0a6de61

Problem: nothing bounds how many questions stand at one height, and a plan passes by being wide instead of deep.
Standard: kanban caps work in progress at every column and stops a pull past the cap.

- A) A cap of four set in the tool; approve and ask stop a plan with a tier over it.
  Why: the harm of one bad question is bounded, and a large plan passes only by being deep.
  Costs: a topic with more than four questions standing on terms alone needs a higher question or a split.
- B) Nothing: a tier holds as many questions as the plan puts in it.
  Why: nothing new in the tool, and the owner reads the plan whole at approve.
  Costs: a plan passes by being wide, and one bad question sits among many at one height.
- C) A cap of four the owner moves with an owner command the ledger records.
  Why: the value moves without a question or a release.
  Costs: one more owner command and ledger event to guard, for a value no plan has pushed against.

**Answer:** A

### Does the tier cap count a decision an open question supersedes?
depends: f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, 2bae531b-c4df-48a0-89a0-3d52726741ea, e2533580-1486-4dc9-8697-e6c53605fe72, 0e38f001-0226-4927-9a78-2e9fc1ec18d4=A|C
supersedes: 14180eab-60a4-48d3-8efc-c1b23a96ce3a
unrelated: de1c5329-e448-45eb-b14c-88b8654447a4, 506b3c6d-3d0a-4a02-98de-3b8a573d2fe9, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, 7327cd58-ac58-40a4-8ef2-00072fb552b2, f8d92841-aef7-4cd7-bd94-0e07c03e4cb6, 9ba58b6a-bc85-44a5-a720-d8e7a6ff7b31, eb2cde37-ebcb-4407-a5b0-9fcda4bd9a4e, 1fb71321-bb84-492f-b9d5-1444c9c444af, dad5273c-afe2-4ab1-a451-e2a5d4d9a26d, 6b3d8e59-e145-49e1-860d-a79d753d745c, f4f667ad-8714-4e65-897b-c7002e423c28, 7c0b9fe3-6038-4a23-93fe-88a31c7eaf40, 49a1c3a8-07ea-448a-8926-2362c0617758, da80931c-68cf-4c51-acce-060307bbd973, 9bf76db8-4821-4257-ac49-4c22c2a0e538, f1b4c4cc-97bd-446f-87b4-df5f1b6f4da2, 010901e0-c02d-49eb-98bc-d584756d7f51, 9afe335c-2a7a-4625-b115-f74d3f091cac, 1d7c758d-1fd5-4e0c-8559-517315b15aaa
id: 86d5e944-ecfa-44bc-9819-83ee4b96890d
terms: approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, move=d904dc0d-231f-4e93-ac74-7babe888107d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, refused=0942e445-0163-423e-8ba7-84dd8ac07045, supersedes=2bae531b-c4df-48a0-89a0-3d52726741ea, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c

Problem: a question replacing a decision in a full tier is refused on the cap when old and new count together, so a decision in a full tier cannot be replaced in its own file.
Standard: a limit on a set is judged on the set as the change leaves it.

- A) No.
  Why: a replacement in a full tier is a legal move, and the file holding a subject keeps holding it.
  Costs: approve and ask count from open supersedes lines, and a refused replacement puts the old back in the count.
- B) Yes.
  Why: one count, read straight off the file.
  Costs: a full tier takes no replacement in its own file, so the replacement lands away from its subject.

**Answer:** A

### What bounds the questions standing on one question?
depends: 277a8338-c57a-43cc-8b28-6b42e9347f6a, aef1b384-2b33-47a2-92a8-ec7f8fb258ef, 9afe335c-2a7a-4625-b115-f74d3f091cac, 010901e0-c02d-49eb-98bc-d584756d7f51
supersedes: 0e38f001-0226-4927-9a78-2e9fc1ec18d4, 86d5e944-ecfa-44bc-9819-83ee4b96890d
unrelated: da80931c-68cf-4c51-acce-060307bbd973, 23f21671-e34c-44aa-8804-6163d975807f, f1b4c4cc-97bd-446f-87b4-df5f1b6f4da2, 1d7c758d-1fd5-4e0c-8559-517315b15aaa, da59b448-1afb-4469-b40e-69eda8f253c2, 506b3c6d-3d0a-4a02-98de-3b8a573d2fe9, 9bf76db8-4821-4257-ac49-4c22c2a0e538, 178a232e-b51c-4122-a1a9-9e9ed6970748, 9c1b476c-9296-444b-a51c-e2957ad40ff5, 4555583f-3499-4963-993b-34556fe519f7, 9d634996-475e-4ddc-8113-05c404ad9d5c, f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, 292a10b8-c287-4d71-b2bd-840da0a6de61
id: b30c29aa-f2b4-4792-9c9f-acc645a29d4e
terms: answered=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, refused=0942e445-0163-423e-8ba7-84dd8ac07045, stands on=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, supersede=2bae531b-c4df-48a0-89a0-3d52726741ea, terms=f590059d-b681-40ec-8a1c-c8916d71f6bb, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, tiers=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c

Problem: a cap on the questions at one height makes unrelated questions share its places, so a question hangs on whichever question leaves a place free.
Standard: an issue tree bounds the branches under each node, a handful per node, and grows by depth where a node fills.

- A) Approve and ask stop a plan where five questions, answered or not, stand on one question or on terms alone.
  Why: a question names the question it stands on, and only a question gathering a fifth is refused.
  Costs: the cap no longer bounds how many questions sit at one height of a file.
- B) Approve and ask stop a plan where five questions, answered or not, sit at one tier.
  Why: one count per height, read off the tiers.
  Costs: unrelated questions share the places of a tier, so a question hangs on whichever question leaves a place free.
- C) Nothing: as many questions stand on one question as the plan writes.
  Why: nothing to count, and the owner reads the plan whole at approve.
  Costs: thirty questions standing on one decision pass, and one supersede drops all of them.

### Does the cap count a decision an open question supersedes?
depends: 2bae531b-c4df-48a0-89a0-3d52726741ea, e2533580-1486-4dc9-8697-e6c53605fe72, b30c29aa-f2b4-4792-9c9f-acc645a29d4e=A|B
supersedes: 86d5e944-ecfa-44bc-9819-83ee4b96890d
unrelated: 24bdcbde-d9f4-45a0-8c3d-37e53569e950, 506b3c6d-3d0a-4a02-98de-3b8a573d2fe9, da80931c-68cf-4c51-acce-060307bbd973, d808917b-0717-45ad-ab83-9fa274b9a9d4, da59b448-1afb-4469-b40e-69eda8f253c2, 9ba58b6a-bc85-44a5-a720-d8e7a6ff7b31, 0e38f001-0226-4927-9a78-2e9fc1ec18d4, a469ecb9-a4f1-4a2a-89c5-7378cc1138b4, f4f667ad-8714-4e65-897b-c7002e423c28, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, dad5273c-afe2-4ab1-a451-e2a5d4d9a26d, b782664f-7465-4e1e-99e0-13f6dd2de774, 48f4647d-8c81-4e8a-a936-6b32f5fb0699, 4555583f-3499-4963-993b-34556fe519f7
id: 9c1b476c-9296-444b-a51c-e2957ad40ff5
terms: approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, move=d904dc0d-231f-4e93-ac74-7babe888107d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, refused=0942e445-0163-423e-8ba7-84dd8ac07045, supersedes=2bae531b-c4df-48a0-89a0-3d52726741ea

Problem: when old and new count together, a decision among four standing on one question cannot be replaced where it stands.
Standard: a limit on a set is judged on the set as the change leaves it.

- A) No.
  Why: a replacement in a full place is a legal move, and the file holding a subject keeps holding it.
  Costs: approve and ask count from open supersedes lines, and a refused replacement puts the old back in the count.
- B) Yes.
  Why: one count, read straight off the file.
  Costs: a full place takes no replacement, so the replacement lands away from its subject.

### Does the cap stop a plan whose questions over it are all answered?
depends: b36113a1-1384-4e4c-a798-8399f4181366, b30c29aa-f2b4-4792-9c9f-acc645a29d4e=A|B, dad5273c-afe2-4ab1-a451-e2a5d4d9a26d
unrelated: 21ae8ae0-6884-4d3c-8464-024a8a5aae2d, da59b448-1afb-4469-b40e-69eda8f253c2, 9ba58b6a-bc85-44a5-a720-d8e7a6ff7b31, 86d5e944-ecfa-44bc-9819-83ee4b96890d, d984e425-3e88-4756-88fb-18241f9f62a7, 010901e0-c02d-49eb-98bc-d584756d7f51, 0e38f001-0226-4927-9a78-2e9fc1ec18d4, fcf9d2c6-009e-4d6a-a3e4-14274bd1ee94, 9bf76db8-4821-4257-ac49-4c22c2a0e538, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, 6b3d8e59-e145-49e1-860d-a79d753d745c, c3127af2-4c96-48d5-a164-bac49a9a09ec, 48f4647d-8c81-4e8a-a936-6b32f5fb0699, 9c1b476c-9296-444b-a51c-e2957ad40ff5, e4a2ef0c-3354-4c9b-a8d9-9e204e544d85
id: 4555583f-3499-4963-993b-34556fe519f7
terms: answered=b36113a1-1384-4e4c-a798-8399f4181366, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, moves=d904dc0d-231f-4e93-ac74-7babe888107d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, stands on=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, supersede=2bae531b-c4df-48a0-89a0-3d52726741ea, superseded=2bae531b-c4df-48a0-89a0-3d52726741ea

Problem: when five answered decisions stand on one question, no edit of the plan clears the cap, since an answered decision never changes.
Standard: a linter run on a change reports what the change touches, not every old line of the file.

- A) No.
  Why: a plan moves while answered decisions over the cap stand; the cap judges them once an open question joins them.
  Costs: a question loaded with answered decisions stays loaded until a new question stands on it or it is superseded.
- B) Yes.
  Why: one rule over the whole file at every run.
  Costs: a file holding such a question stops every plan until a supersede, and each decision standing on it falls too.
