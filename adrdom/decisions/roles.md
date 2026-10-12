# Roles

What the tool knows a session by, what makes a session a planner, and what the tool
holds a planner from once it is one.

### What does the tool know a session by?
depends: 3afef03f-9017-4638-9759-e010d44d536b, f90dbd5b-8fbe-4899-8588-f8762795c2e2, e877e4a9-2caf-4fe6-9f82-5476a2abb00b, 98d5d389-8315-41da-9970-666e31f3a94d, ae63df95-0643-4108-af65-1e7a0009d631
unrelated: 36dfbeaa-698d-4a57-b3b0-d425348e14a5, 4838baa4-a5df-49e7-929c-ed63b5adaf4e, 2db33118-82b4-4edf-a9cb-e92ac62738b9, 4b407ee1-1e64-4ca1-842b-69773fc0e48a, 1ab6d6c3-7e44-42de-bc2f-2e5849114aef, aa475aa3-05b2-429a-b33a-46b1dcb52049, 08a1b488-32fc-4dc7-b4b3-93a6e768725f, 209081f4-864f-4ef6-8909-65da87f135e5, 7127ab1e-c313-4c51-9134-c09d551682cf, 6c406624-b874-4697-9a44-91941019b7d1, 2381d759-f85f-484e-9031-fadbe6ea045a, 1cec44fc-2685-42f8-9742-7facabb52251, 1ee664a1-cac0-4917-9e46-8199a387d235, 1259323f-cdec-4101-9d47-a165a286089f, de1c5329-e448-45eb-b14c-88b8654447a4, 8d3337b9-6fb3-4756-80f1-b388fe81d856, 8081c82d-e5c6-485b-a302-07d586f1a01b, db07a54d-4d80-4035-a9ef-eabcc0c872b3, 8f81dee1-9c19-447d-baa8-d7d0af691db8, b0e865b4-9204-4c55-9f5b-dcb755b3a03b, 4f04bae4-98e3-48eb-9403-3257268e2d87, 34e81f57-9b3c-4dc4-a2ba-ca227523177d, 4f925b13-ad16-48b8-acd1-3e30d6e308b6, c3127af2-4c96-48d5-a164-bac49a9a09ec, adee66f2-c8ff-477f-8ac4-a497f694c912, ec0b9293-916e-4ced-a3d7-3b862ab7b069, d8a9dfff-de89-442c-a08c-8276efab3f36
id: e53b805c-9272-4b89-985d-61426fb5fa48
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, session=3afef03f-9017-4638-9759-e010d44d536b

Problem: the tool holds a rule on the session that asked a question, and nothing it reads tells one session from another.
Standard: a process identifies its caller by a credential the caller's runtime supplies, read once per invocation.

- A) The identifier the agent program gives its session, read from the environment the command runs in.
  Why: one value that every command and the commit hook see alike, in any checkout or worktree.
  Costs: one variable name per agent program, a list that grows per agent; an agent that unsets it runs unseen.
- B) The checkout the command runs in.
  Why: a path the tool has anyway, with nothing read from the agent program.
  Costs: a new worktree starts with no role, so one session holds two roles in two checkouts.
- C) The author git puts on the commit.
  Why: a value git carries with the commit.
  Costs: one author for every session on a machine, and no author at a command.

**Answer:** A

### What makes a session a planner?
depends: 3aedf314-f25f-49cb-b7b3-d2302e4543ca, 3afef03f-9017-4638-9759-e010d44d536b, 94677ed8-eaf6-46e2-bc71-7392fa0805bf, 3c8d4c9c-abca-4faa-b7c1-aae39951376c, e53b805c-9272-4b89-985d-61426fb5fa48=*, 9ba58b6a-bc85-44a5-a720-d8e7a6ff7b31, 46fc51ff-66b9-4841-a953-ae0d4c2fc424
unrelated: d3be71e4-ce7b-43b0-86a8-e86868521f99, 0a742360-83c2-479d-b858-4bea646c9b36, 21cdad59-4f0c-42a5-b4f3-6af946637df3, 3dba486c-a399-4ce3-a794-e38c4f733403, c3127af2-4c96-48d5-a164-bac49a9a09ec, 23d53545-c238-48cc-ae5e-21a2c7aab83e, 4838baa4-a5df-49e7-929c-ed63b5adaf4e, 2381d759-f85f-484e-9031-fadbe6ea045a, 1cec44fc-2685-42f8-9742-7facabb52251, 1ee664a1-cac0-4917-9e46-8199a387d235, 1259323f-cdec-4101-9d47-a165a286089f, de1c5329-e448-45eb-b14c-88b8654447a4, 8d3337b9-6fb3-4756-80f1-b388fe81d856, 8081c82d-e5c6-485b-a302-07d586f1a01b, db07a54d-4d80-4035-a9ef-eabcc0c872b3, 8f81dee1-9c19-447d-baa8-d7d0af691db8, 6b3d8e59-e145-49e1-860d-a79d753d745c, f8d92841-aef7-4cd7-bd94-0e07c03e4cb6, 98d5d389-8315-41da-9970-666e31f3a94d, ae63df95-0643-4108-af65-1e7a0009d631, ec0b9293-916e-4ced-a3d7-3b862ab7b069
id: 6c406624-b874-4697-9a44-91941019b7d1
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, asking=3c8d4c9c-abca-4faa-b7c1-aae39951376c, asks=3c8d4c9c-abca-4faa-b7c1-aae39951376c, neighbours=daec9ffd-676f-4111-a359-2827bfd4527c, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, planner=3aedf314-f25f-49cb-b7b3-d2302e4543ca, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, reader=be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, session=3afef03f-9017-4638-9759-e010d44d536b, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: the rule that the asking agent does not build holds nothing while the tool cannot tell which session asked.
Standard: a role is assigned at the first act that belongs to it, by the system that observes the act.

- A) Running plan or ask on a topic file.
  Why: both acts put questions on the record; a session that ran either wrote what another builds.
  Costs: a session that runs plan to read a draft's state becomes a planner.
- B) Running plan on a topic file.
  Why: the first act of a round; ask follows it in the same session.
  Costs: a session that asks a plan the owner wrote is no planner.
- C) Running plan, ask or read.
  Why: every act that writes the reader's word marks the session.
  Costs: a session that reads neighbours for a review becomes a planner.

**Answer:** A

### What does check do with a test that names a decision that is not current?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, e2533580-1486-4dc9-8697-e6c53605fe72, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, c49997ee-10e7-48e0-b66b-43f8b09758a6, 6c406624-b874-4697-9a44-91941019b7d1=*, 4319a653-6901-446c-94ab-6420ef472134, 3460d220-e523-448b-aeff-baa8051b7d06
supersedes: 751ef30c-6a2f-4936-bf9b-b9d487146e53, 23d55c81-2224-487c-a579-298915957a79, 110f471d-ac4b-4cbb-aa7b-3f74ac442d78
unrelated: c10c3a49-e993-4c9f-a46b-4a1ba3646dcf, e3200081-9995-4bbe-983f-2f430dc341e7, 3fc35d4f-90f3-4425-9dac-951a89631fdf, ee27fd4c-89b7-411f-a3e4-2535c49b6d1b, c3127af2-4c96-48d5-a164-bac49a9a09ec, e53b805c-9272-4b89-985d-61426fb5fa48, 6c406624-b874-4697-9a44-91941019b7d1, 1cec44fc-2685-42f8-9742-7facabb52251, 1ee664a1-cac0-4917-9e46-8199a387d235, 1259323f-cdec-4101-9d47-a165a286089f, de1c5329-e448-45eb-b14c-88b8654447a4, 8d3337b9-6fb3-4756-80f1-b388fe81d856, 8081c82d-e5c6-485b-a302-07d586f1a01b, db07a54d-4d80-4035-a9ef-eabcc0c872b3, 8f81dee1-9c19-447d-baa8-d7d0af691db8, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, bf6697e3-8832-4b90-b036-8fecedb55406, 76c6adc6-c018-4df4-b49a-ddcd0a03a48c, a26876ba-5789-4d05-965e-853d00d9edd2, 05141f49-d9d2-407c-b60c-ebb195a05864, 0c3d498a-c540-47f3-8384-050fc044c109, bc4291c8-98f1-499d-8e02-46e371035e70
id: 2381d759-f85f-484e-9031-fadbe6ea045a
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, builder=c49997ee-10e7-48e0-b66b-43f8b09758a6, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, finding=ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, move=d904dc0d-231f-4e93-ac74-7babe888107d, moves=d904dc0d-231f-4e93-ac74-7babe888107d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, planner=3aedf314-f25f-49cb-b7b3-d2302e4543ca, session=3afef03f-9017-4638-9759-e010d44d536b, superseded=2bae531b-c4df-48a0-89a0-3d52726741ea

Problem: a tie to a superseded decision fails check at the answer commit, and the session that commits the answer may not touch the test.
Standard: a stale traceability link is a finding assigned to the owner of the code, not a block on the requirement change.

- A) Reports it as a finding whose move is the builder's, and fails nothing.
  Why: an answer commits with the ties as they stand; the builder moves each tie to the current decision.
  Costs: a tie claims a decision out of force until a builder commits.
- B) Fails it.
  Why: every tie on the record names a decision in force.
  Costs: the answer commit waits on a test edit, which is a build.
- C) Reports it in a planner's session and fails it in any other.
  Why: the planner commits the answer; the builder cannot commit past a stale tie.
  Costs: two outcomes of one finding by who runs check.

**Answer:** A

### Does the ledger hold the planners?
depends: e18b82d8-5271-4385-bbab-2b89e5da278f, 3aedf314-f25f-49cb-b7b3-d2302e4543ca, e53b805c-9272-4b89-985d-61426fb5fa48=*, 6c406624-b874-4697-9a44-91941019b7d1=*, f3e64c4e-21ab-442f-8c40-6060591e2b77, 16a71892-b348-4b9b-82c5-a9d375187bd3
unrelated: d3be71e4-ce7b-43b0-86a8-e86868521f99, bf6697e3-8832-4b90-b036-8fecedb55406, cd83e277-ebf0-4458-a77a-c526ab4cfeca, 06b7e003-4d9f-427a-a798-c1af97865411, 26c5352f-3f82-4376-9dc2-8ccc0074d080, a26876ba-5789-4d05-965e-853d00d9edd2, 3dba486c-a399-4ce3-a794-e38c4f733403, f422c5db-13d0-4ff7-9e43-b68ab44a7cb9, 385a57d5-5f76-454a-ae5d-9b0d6f5c1492, 2381d759-f85f-484e-9031-fadbe6ea045a, 1ee664a1-cac0-4917-9e46-8199a387d235, 1259323f-cdec-4101-9d47-a165a286089f, de1c5329-e448-45eb-b14c-88b8654447a4, 8d3337b9-6fb3-4756-80f1-b388fe81d856, 8081c82d-e5c6-485b-a302-07d586f1a01b, db07a54d-4d80-4035-a9ef-eabcc0c872b3, 8f81dee1-9c19-447d-baa8-d7d0af691db8, dfd32ef6-c2ac-40ef-b8b3-9c58732516b1, 76c6adc6-c018-4df4-b49a-ddcd0a03a48c, 99619e27-19b6-47ef-83b9-c79ac30ee2f0, db56a285-946f-48b1-b073-bb1e90192185
id: 1cec44fc-2685-42f8-9742-7facabb52251
terms: identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, planner=3aedf314-f25f-49cb-b7b3-d2302e4543ca, planners=3aedf314-f25f-49cb-b7b3-d2302e4543ca, session=3afef03f-9017-4638-9759-e010d44d536b

Problem: the role has to outlive the command that assigns it, and the ledger is the one record the tool appends to.
Standard: per-machine runtime state stays out of version control.

- A) No.
  Why: the role lives in an untracked file under adrdom/state, holding the identifier and the word.
  Costs: a second store beside the ledger; a fresh clone or a new worktree knows no planner.
- B) Yes.
  Why: one record; a planner's role reaches every clone.
  Costs: a new ledger event, a row per session, and a role that means nothing off its machine.

**Answer:** A


### What ends a session's planner role?
depends: 3aedf314-f25f-49cb-b7b3-d2302e4543ca, 3afef03f-9017-4638-9759-e010d44d536b, e53b805c-9272-4b89-985d-61426fb5fa48=A, 6c406624-b874-4697-9a44-91941019b7d1=*, 9ba58b6a-bc85-44a5-a720-d8e7a6ff7b31
unrelated: d3be71e4-ce7b-43b0-86a8-e86868521f99, 21ae8ae0-6884-4d3c-8464-024a8a5aae2d, da59b448-1afb-4469-b40e-69eda8f253c2, 21cdad59-4f0c-42a5-b4f3-6af946637df3, 3dba486c-a399-4ce3-a794-e38c4f733403, c3127af2-4c96-48d5-a164-bac49a9a09ec, 46fc51ff-66b9-4841-a953-ae0d4c2fc424, 79da6500-2246-4a80-baf1-0eaf7cad49ee, e4a2ef0c-3354-4c9b-a8d9-9e204e544d85, e53b805c-9272-4b89-985d-61426fb5fa48, 2381d759-f85f-484e-9031-fadbe6ea045a, 1cec44fc-2685-42f8-9742-7facabb52251, 1259323f-cdec-4101-9d47-a165a286089f, de1c5329-e448-45eb-b14c-88b8654447a4, 8d3337b9-6fb3-4756-80f1-b388fe81d856, 8081c82d-e5c6-485b-a302-07d586f1a01b, db07a54d-4d80-4035-a9ef-eabcc0c872b3, 8f81dee1-9c19-447d-baa8-d7d0af691db8, 0a742360-83c2-479d-b858-4bea646c9b36, f8d92841-aef7-4cd7-bd94-0e07c03e4cb6, 23d53545-c238-48cc-ae5e-21a2c7aab83e, 98d5d389-8315-41da-9970-666e31f3a94d, 6b3d8e59-e145-49e1-860d-a79d753d745c, 209081f4-864f-4ef6-8909-65da87f135e5
id: 1ee664a1-cac0-4917-9e46-8199a387d235
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answered=b36113a1-1384-4e4c-a798-8399f4181366, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, asking=3c8d4c9c-abca-4faa-b7c1-aae39951376c, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, planner=3aedf314-f25f-49cb-b7b3-d2302e4543ca, plans=94677ed8-eaf6-46e2-bc71-7392fa0805bf, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, session=3afef03f-9017-4638-9759-e010d44d536b, topic=292a10b8-c287-4d71-b2bd-840da0a6de61, topics=292a10b8-c287-4d71-b2bd-840da0a6de61

Problem: a planner that may build once its questions are answered is the asking agent building its decision.
Standard: a role bound to a process ends with the process.

- A) The end of the session; the role stays on file and a new session brings a new identifier.
  Why: one session, one role; the planner goes on to other topics as a planner.
  Costs: a planner that would build another topic opens a new session.
- B) The approval of the plan the session wrote: it then builds topics it did not ask.
  Why: a session plans once and builds elsewhere after.
  Costs: the hook must tell one topic's build from another's, and a commit names no decision.

**Answer:** A

### What does a comment in code carry to name the decision it follows?
depends: 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, e877e4a9-2caf-4fe6-9f82-5476a2abb00b, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, f90dbd5b-8fbe-4899-8588-f8762795c2e2, 4319a653-6901-446c-94ab-6420ef472134, 3460d220-e523-448b-aeff-baa8051b7d06, 2381d759-f85f-484e-9031-fadbe6ea045a=*
unrelated: f53ac3b9-efde-427c-8027-19988aa8fda7, 946263aa-519a-47aa-b743-5cdcf52f5d35, 110f471d-ac4b-4cbb-aa7b-3f74ac442d78, 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, 751ef30c-6a2f-4936-bf9b-b9d487146e53, 6108b8b2-911e-443b-abe0-4c6f8b407384, ece60954-56d4-4622-883e-265f3b997c9f, c3127af2-4c96-48d5-a164-bac49a9a09ec, 0487a1cf-eb61-4bf8-bd16-eed899d22d2c, 05141f49-d9d2-407c-b60c-ebb195a05864, ee27fd4c-89b7-411f-a3e4-2535c49b6d1b, 23d55c81-2224-487c-a579-298915957a79, aef1b384-2b33-47a2-92a8-ec7f8fb258ef, 2bae531b-c4df-48a0-89a0-3d52726741ea, e53b805c-9272-4b89-985d-61426fb5fa48, 6c406624-b874-4697-9a44-91941019b7d1, 1cec44fc-2685-42f8-9742-7facabb52251, 1ee664a1-cac0-4917-9e46-8199a387d235, de1c5329-e448-45eb-b14c-88b8654447a4, 8d3337b9-6fb3-4756-80f1-b388fe81d856, 8081c82d-e5c6-485b-a302-07d586f1a01b, db07a54d-4d80-4035-a9ef-eabcc0c872b3, 8f81dee1-9c19-447d-baa8-d7d0af691db8
id: 1259323f-cdec-4101-9d47-a165a286089f
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, checks=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, finding=ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862

Problem: a comment adr: <id> is the test of a decision wherever it stands, so the same comment on the code that follows the decision clears the untested finding without a test.
Standard: a code annotation names the requirement it implements and is distinct from the test that proves it.

- A) A comment adr-code: <id>, found by text search, never a test tie, judged by no rule.
  Why: an agent finds the code behind a decision with one search, and the comment can clear nothing.
  Costs: a claim no gate checks; a stale or wrong comment is caught at review alone.
- B) The same comment adr: <id> a test carries, counted as the test.
  Why: one comment form for test and code.
  Costs: a comment on code clears the untested finding, and a decision passes as tested with no test.
- C) Nothing: code carries no decision name.
  Why: the record names code through tests alone.
  Costs: an agent building on a decision reads the code with no mark of what follows it.

**Answer:** A

### Does a finding name the identifier of the decision it judges?
depends: ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, e877e4a9-2caf-4fe6-9f82-5476a2abb00b, e2533580-1486-4dc9-8697-e6c53605fe72, 2bae531b-c4df-48a0-89a0-3d52726741ea, 4319a653-6901-446c-94ab-6420ef472134, 184b5bcb-fa72-436b-92cb-4d4605366c59, 2381d759-f85f-484e-9031-fadbe6ea045a=*
unrelated: 946263aa-519a-47aa-b743-5cdcf52f5d35, 7a1e7b81-0967-4ed2-8a8a-c7153056836f, 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, c6da863d-5085-402f-815a-9bb832b4df7e, 751ef30c-6a2f-4936-bf9b-b9d487146e53, 6108b8b2-911e-443b-abe0-4c6f8b407384, de5ed5b1-3a83-4403-8953-941cb7e94390, c3127af2-4c96-48d5-a164-bac49a9a09ec, 14180eab-60a4-48d3-8efc-c1b23a96ce3a, 4b407ee1-1e64-4ca1-842b-69773fc0e48a, 7c306235-6243-4d79-b9aa-08ed19662779, 3460d220-e523-448b-aeff-baa8051b7d06, 110f471d-ac4b-4cbb-aa7b-3f74ac442d78, ee27fd4c-89b7-411f-a3e4-2535c49b6d1b, 125ee772-d011-4dab-b53e-97a09ec03198, 23d55c81-2224-487c-a579-298915957a79, e53b805c-9272-4b89-985d-61426fb5fa48, 6c406624-b874-4697-9a44-91941019b7d1, 1cec44fc-2685-42f8-9742-7facabb52251, 1ee664a1-cac0-4917-9e46-8199a387d235, 1259323f-cdec-4101-9d47-a165a286089f, 8d3337b9-6fb3-4756-80f1-b388fe81d856, 8081c82d-e5c6-485b-a302-07d586f1a01b, db07a54d-4d80-4035-a9ef-eabcc0c872b3, 8f81dee1-9c19-447d-baa8-d7d0af691db8, d2851ea1-fcb6-476a-b321-c813a594d5e5
id: de1c5329-e448-45eb-b14c-88b8654447a4
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

### Does the tool mark a builder?
depends: c49997ee-10e7-48e0-b66b-43f8b09758a6, 3aedf314-f25f-49cb-b7b3-d2302e4543ca, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, b36113a1-1384-4e4c-a798-8399f4181366, 6c406624-b874-4697-9a44-91941019b7d1=*, 1ee664a1-cac0-4917-9e46-8199a387d235=*, c3127af2-4c96-48d5-a164-bac49a9a09ec, 3c8d4c9c-abca-4faa-b7c1-aae39951376c
unrelated: 99619e27-19b6-47ef-83b9-c79ac30ee2f0, 0d977e39-a657-4596-8eca-8ceb335f3081, 125ee772-d011-4dab-b53e-97a09ec03198, 48f4647d-8c81-4e8a-a936-6b32f5fb0699, 4838baa4-a5df-49e7-929c-ed63b5adaf4e, 98d5d389-8315-41da-9970-666e31f3a94d, 2db33118-82b4-4edf-a9cb-e92ac62738b9, aa475aa3-05b2-429a-b33a-46b1dcb52049, 7127ab1e-c313-4c51-9134-c09d551682cf, e53b805c-9272-4b89-985d-61426fb5fa48, 2381d759-f85f-484e-9031-fadbe6ea045a, 1cec44fc-2685-42f8-9742-7facabb52251, 1259323f-cdec-4101-9d47-a165a286089f, de1c5329-e448-45eb-b14c-88b8654447a4, 8081c82d-e5c6-485b-a302-07d586f1a01b, db07a54d-4d80-4035-a9ef-eabcc0c872b3, 8f81dee1-9c19-447d-baa8-d7d0af691db8, dfd32ef6-c2ac-40ef-b8b3-9c58732516b1, 76c6adc6-c018-4df4-b49a-ddcd0a03a48c, db56a285-946f-48b1-b073-bb1e90192185, 4f925b13-ad16-48b8-acd1-3e30d6e308b6, f8d92841-aef7-4cd7-bd94-0e07c03e4cb6, 23d53545-c238-48cc-ae5e-21a2c7aab83e, e879fe72-ec07-48cd-9902-2b158aaae726, 05141f49-d9d2-407c-b60c-ebb195a05864, b782664f-7465-4e1e-99e0-13f6dd2de774
id: 8d3337b9-6fb3-4756-80f1-b388fe81d856
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, asks=3c8d4c9c-abca-4faa-b7c1-aae39951376c, builder=c49997ee-10e7-48e0-b66b-43f8b09758a6, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, refuse=0942e445-0163-423e-8ba7-84dd8ac07045, session=3afef03f-9017-4638-9759-e010d44d536b

Problem: a session that commits code and then writes the questions on what it committed asks for its own build.
Standard: a gate is placed where no other gate stands.

- A) No.
  Why: a question from any session passes approve and answer before it binds; the owner reads it there.
  Costs: a builder may write the questions on what it built, and the owner catches it reading.
- B) Yes.
  Why: the first commit outside adrdom marks the session, and plan and ask refuse it after.
  Costs: a second mark, a hook that writes, and a read in plan and ask, before gates the owner holds anyway.

**Answer:** A


### Does a command stop when the environment gives no session identifier?
depends: 3afef03f-9017-4638-9759-e010d44d536b, e877e4a9-2caf-4fe6-9f82-5476a2abb00b, 1d7e59ea-99e3-4d4b-9928-5dabca62723d, e53b805c-9272-4b89-985d-61426fb5fa48=A, 1cec44fc-2685-42f8-9742-7facabb52251=*, aa275fac-def5-42a2-8a61-660142acad8e, 98d5d389-8315-41da-9970-666e31f3a94d, b0e865b4-9204-4c55-9f5b-dcb755b3a03b, dfd32ef6-c2ac-40ef-b8b3-9c58732516b1
unrelated: aac5c313-dedd-466b-a0c3-ead5e99ce913, f53ac3b9-efde-427c-8027-19988aa8fda7, 36dfbeaa-698d-4a57-b3b0-d425348e14a5, fe14991d-9715-4ae0-9a23-d314a950d3b9, 3fc35d4f-90f3-4425-9dac-951a89631fdf, 902bd279-e6c1-4a21-9637-a3a43f360bae, 848e1b76-9cfe-483b-94dd-094eb9b2f995, 069a6b4a-de35-45f3-8c6c-8e254bf40565, 6c406624-b874-4697-9a44-91941019b7d1, 2381d759-f85f-484e-9031-fadbe6ea045a, 1ee664a1-cac0-4917-9e46-8199a387d235, 1259323f-cdec-4101-9d47-a165a286089f, de1c5329-e448-45eb-b14c-88b8654447a4, 8d3337b9-6fb3-4756-80f1-b388fe81d856, db07a54d-4d80-4035-a9ef-eabcc0c872b3, 8f81dee1-9c19-447d-baa8-d7d0af691db8, 4f925b13-ad16-48b8-acd1-3e30d6e308b6, c3127af2-4c96-48d5-a164-bac49a9a09ec, f422c5db-13d0-4ff7-9e43-b68ab44a7cb9, ae63df95-0643-4108-af65-1e7a0009d631, 48f4647d-8c81-4e8a-a936-6b32f5fb0699
id: 8081c82d-e5c6-485b-a302-07d586f1a01b
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plans=94677ed8-eaf6-46e2-bc71-7392fa0805bf, session=3afef03f-9017-4638-9759-e010d44d536b

Problem: the owner's terminal and an agent program the tool does not know give no identifier.
Standard: an optional credential is absent, not invalid; absence takes the open path.

- A) No.
  Why: a session the tool cannot name is held by nothing, the owner's own commits among them.
  Costs: an agent program that gives no identifier plans and builds in one session unseen.
- B) Yes.
  Why: every session that plans is known; none plans unseen.
  Costs: the owner, every test and the push check run no command, and the hook stops the owner's commits.

**Answer:** A


### Does check fail a planner's commit that reaches outside adrdom?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, 3aedf314-f25f-49cb-b7b3-d2302e4543ca, e53b805c-9272-4b89-985d-61426fb5fa48=A, 6c406624-b874-4697-9a44-91941019b7d1=*, 1ee664a1-cac0-4917-9e46-8199a387d235=*, 1cec44fc-2685-42f8-9742-7facabb52251=*, 8081c82d-e5c6-485b-a302-07d586f1a01b=*, 9ba58b6a-bc85-44a5-a720-d8e7a6ff7b31, 98d5d389-8315-41da-9970-666e31f3a94d, 4f925b13-ad16-48b8-acd1-3e30d6e308b6
unrelated: bc4291c8-98f1-499d-8e02-46e371035e70, 1c27f2d6-ab62-4c5a-91b2-1488722f3c48, c10c3a49-e993-4c9f-a46b-4a1ba3646dcf, 751ef30c-6a2f-4936-bf9b-b9d487146e53, 912f4f3b-1213-435a-af0d-7617017fcff4, 6ef9eca6-fd8b-4211-aae7-00b37a2a9245, 76c6adc6-c018-4df4-b49a-ddcd0a03a48c, a26876ba-5789-4d05-965e-853d00d9edd2, 19e1c023-a678-4842-a414-bb0e45c41601, 05141f49-d9d2-407c-b60c-ebb195a05864, e53b805c-9272-4b89-985d-61426fb5fa48, 2381d759-f85f-484e-9031-fadbe6ea045a, 1259323f-cdec-4101-9d47-a165a286089f, de1c5329-e448-45eb-b14c-88b8654447a4, 8d3337b9-6fb3-4756-80f1-b388fe81d856, 8081c82d-e5c6-485b-a302-07d586f1a01b, 8f81dee1-9c19-447d-baa8-d7d0af691db8, db56a285-946f-48b1-b073-bb1e90192185, c3127af2-4c96-48d5-a164-bac49a9a09ec
id: db07a54d-4d80-4035-a9ef-eabcc0c872b3
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, asking=3c8d4c9c-abca-4faa-b7c1-aae39951376c, brief=1d9df88a-40f6-4235-a083-e063fde634fe, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, planner=3aedf314-f25f-49cb-b7b3-d2302e4543ca, refused=0942e445-0163-423e-8ba7-84dd8ac07045, session=3afef03f-9017-4638-9759-e010d44d536b

Problem: the record says the asking agent does not build, and nothing holds the commit that breaks it.
Standard: a policy on changes is enforced at the commit hook, where a change is visible as paths.

- A) Yes.
  Why: everything a planner writes lives under adrdom; a path outside it in a planner's commit is a build.
  Costs: a planner that commits a brief edit, a tie or an install is refused; a new session does that.
- B) No.
  Why: the brief alone holds the rule, as it holds every rule on the agent's conduct.
  Costs: the rule stays unenforced; the asking agent builds its decision with nothing in the way.

**Answer:** A

### What does the tool print for a build move in a planner's session?
depends: d904dc0d-231f-4e93-ac74-7babe888107d, 3aedf314-f25f-49cb-b7b3-d2302e4543ca, c49997ee-10e7-48e0-b66b-43f8b09758a6, 6c406624-b874-4697-9a44-91941019b7d1=*, db07a54d-4d80-4035-a9ef-eabcc0c872b3=A, c84a4bd9-fc2d-44f0-b333-172978f61544, a6f41dee-2381-421a-bcc4-b14af243209b, aac5c313-dedd-466b-a0c3-ead5e99ce913
unrelated: 36dfbeaa-698d-4a57-b3b0-d425348e14a5, d3be71e4-ce7b-43b0-86a8-e86868521f99, 3fc35d4f-90f3-4425-9dac-951a89631fdf, bceceda8-133a-4c9b-b439-b5c1a4414a02, c3127af2-4c96-48d5-a164-bac49a9a09ec, 2db33118-82b4-4edf-a9cb-e92ac62738b9, 385a57d5-5f76-454a-ae5d-9b0d6f5c1492, 7857d633-2055-47eb-857f-cb70da9fa7ba, 05141f49-d9d2-407c-b60c-ebb195a05864, e53b805c-9272-4b89-985d-61426fb5fa48, 2381d759-f85f-484e-9031-fadbe6ea045a, 1cec44fc-2685-42f8-9742-7facabb52251, 1ee664a1-cac0-4917-9e46-8199a387d235, 1259323f-cdec-4101-9d47-a165a286089f, de1c5329-e448-45eb-b14c-88b8654447a4, 8d3337b9-6fb3-4756-80f1-b388fe81d856, 8081c82d-e5c6-485b-a302-07d586f1a01b, bc4291c8-98f1-499d-8e02-46e371035e70, 6b3d8e59-e145-49e1-860d-a79d753d745c, 34e81f57-9b3c-4dc4-a2ba-ca227523177d, 23d53545-c238-48cc-ae5e-21a2c7aab83e, f422c5db-13d0-4ff7-9e43-b68ab44a7cb9, d8a9dfff-de89-442c-a08c-8276efab3f36, 4838baa4-a5df-49e7-929c-ed63b5adaf4e, 209081f4-864f-4ef6-8909-65da87f135e5, 48f4647d-8c81-4e8a-a936-6b32f5fb0699, 98d5d389-8315-41da-9970-666e31f3a94d
id: 8f81dee1-9c19-447d-baa8-d7d0af691db8
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, builder=c49997ee-10e7-48e0-b66b-43f8b09758a6, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, move=d904dc0d-231f-4e93-ac74-7babe888107d, moves=d904dc0d-231f-4e93-ac74-7babe888107d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, planner=3aedf314-f25f-49cb-b7b3-d2302e4543ca, reader=be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, refusal=0942e445-0163-423e-8ba7-84dd8ac07045, refused=0942e445-0163-423e-8ba7-84dd8ac07045, session=3afef03f-9017-4638-9759-e010d44d536b

Problem: a planner reads a build move as its own, builds, and is refused at the commit, after the work.
Standard: a listing tells the reader which items are theirs before they act.

- A) The move with builder in place of agent, on every builds line show and check print.
  Why: the planner reads that the move is another session's before any work.
  Costs: the move reads the role file, and the mover gains a builder beside agent and owner.
- B) The move as in any session.
  Why: one output whoever runs show or check.
  Costs: the refusal comes at the commit, after the build.
- C) No build move.
  Why: the planner sees its own moves alone.
  Costs: the planner cannot read what waits on a builder, and check hides untested decisions from it.

**Answer:** A

### Does a finding hold the identifier of the decision it judges?
depends: finding, decision, identifier, What does check do with a test that names a decision that is not current?
supersedes: Does a finding name the identifier of the decision it judges?
unrelated: Does a depends entry on a question in its file carry an answer set?, Does a question keep its identifier when its heading is reworded?, Does a reference in a topic file carry the identifier of what it names?, Does a supersedes line name every current decision standing on its target?, Does a term carry its own banned words?, Does check read the words of an answered question again?, Does every sentence of the brief come from a decision?, Does the agent build what no current decision covers?, Does the cap count a decision that an open question supersedes?, Does the tool read a pair of decisions in both directions?, What does a term hold?, What does a test carry to name its decision?, What does check do with an untested current decision?, current, supersede, What does a comment in code carry to name the decision it follows?, What does a finding name?, What does check do with a decision row that no question names?, What does a blame file hold of its origin?

Problem: a finding is tagged with a name the code picks, tells, plan, ids; nothing ties a message to the decision it judges, and a message with no decision behind it is written like the rest.
Standard: every diagnostic carries a stable code that documentation and tests key on.

- A) Yes.
  Why: a finding with no current decision behind it cannot be built; one test holds every finding to its decision.
  Costs: the identifier stays in the code and the test, never on the line; a superseded decision re-tags its findings.
- B) No.
  Why: the code names its rules as it likes; the record and the messages stay apart.
  Costs: a message with no decision behind it passes every gate and is caught at review alone.
