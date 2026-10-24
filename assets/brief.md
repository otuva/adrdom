# adrdom

This repository is governed by `adr`. What the code does is a decision: the option the owner picked when they answered a question. <!-- from: c3127af2-4c96-48d5-a164-bac49a9a09ec, 125ee772-d011-4dab-b53e-97a09ec03198, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, b36113a1-1384-4e4c-a798-8399f4181366 -->
Agents write the questions. Only the owner approves a plan and answers a question, at their own terminal. <!-- from: 23d53545-c238-48cc-ae5e-21a2c7aab83e, aa275fac-def5-42a2-8a61-660142acad8e, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, b36113a1-1384-4e4c-a798-8399f4181366 -->
The git hooks run `adr check` at every commit and push; check refuses what breaks a decision. <!-- from: 34e81f57-9b3c-4dc4-a2ba-ca227523177d, 19e1c023-a678-4842-a414-bb0e45c41601, 4f925b13-ad16-48b8-acd1-3e30d6e308b6, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de -->

## agents

### Roles

A session that runs `adr plan` is a planner until it ends: it does not build the decisions it planned, and its commits hold only paths under `adrdom/`. <!-- from: 3afef03f-9017-4638-9759-e010d44d536b, f90dbd5b-8fbe-4899-8588-f8762795c2e2, 3aedf314-f25f-49cb-b7b3-d2302e4543ca, 6c406624-b874-4697-9a44-91941019b7d1, 1ee664a1-cac0-4917-9e46-8199a387d235, 9ba58b6a-bc85-44a5-a720-d8e7a6ff7b31, db07a54d-4d80-4035-a9ef-eabcc0c872b3, 3762e14b-6dfe-45ee-b407-d2718e817348 -->
A builder is another session; it builds from the record alone. <!-- from: c49997ee-10e7-48e0-b66b-43f8b09758a6, 9ba58b6a-bc85-44a5-a720-d8e7a6ff7b31, 4319a653-6901-446c-94ab-6420ef472134, 3460d220-e523-448b-aeff-baa8051b7d06 -->
A subagent runs under the session's identifier and carries its role. With no session identifier in the environment, no role applies. <!-- from: e53b805c-9272-4b89-985d-61426fb5fa48, 6c406624-b874-4697-9a44-91941019b7d1, 3afef03f-9017-4638-9759-e010d44d536b, 8081c82d-e5c6-485b-a302-07d586f1a01b -->
The owner's commands are `adr approve`, `adr answer`, `adr unban`, `adr serve` and `adr uninstall`; they refuse an agent. Every other command is the agent's. <!-- from: aa275fac-def5-42a2-8a61-660142acad8e, 292359a4-0512-465b-b6a7-79838e66e5a5, ef7ec04d-9cdd-40a1-8246-42a4a0200828, ece60954-56d4-4622-883e-265f3b997c9f, df452a5a-1e2d-497e-93e9-7d6a80e8540d, ae63df95-0643-4108-af65-1e7a0009d631, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, b36113a1-1384-4e4c-a798-8399f4181366, 1d7e59ea-99e3-4d4b-9928-5dabca62723d -->

### Moves

`adr show` prints every current decision, then the findings and the moves; a clean repository prints decisions only. <!-- from: c5e10ae8-258d-4f29-b9fe-13cb7ad2a550, 1af76f91-5644-42ef-97ea-8f70f934c26c -->
A finding is what the tool found and the one move that clears it. A move opens with who makes it: <!-- from: 50b31713-4fea-4aa3-842a-79c2750660ba, ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, b0e865b4-9204-4c55-9f5b-dcb755b3a03b, d904dc0d-231f-4e93-ac74-7babe888107d, 9a2ff21b-c20b-4c63-95f2-a7a3cbe236b8 -->
- `agent`: make it, in the order printed. <!-- from: d904dc0d-231f-4e93-ac74-7babe888107d, 9a2ff21b-c20b-4c63-95f2-a7a3cbe236b8 -->
- `builder`: another session's; leave it. <!-- from: d904dc0d-231f-4e93-ac74-7babe888107d, 9a2ff21b-c20b-4c63-95f2-a7a3cbe236b8, 9ba58b6a-bc85-44a5-a720-d8e7a6ff7b31 -->
- `owner runs: <command>`: post the command in chat and stop. <!-- from: d904dc0d-231f-4e93-ac74-7babe888107d, b0e865b4-9204-4c55-9f5b-dcb755b3a03b -->
- `not yet`: its turn has not come; leave it. <!-- from: d904dc0d-231f-4e93-ac74-7babe888107d, f8d92841-aef7-4cd7-bd94-0e07c03e4cb6 -->
A refusal names what failed and the move that clears it; make that move and no other. The owner is not a way past a refusal: never change a file so that the owner's command clears it, and never ask for an approval or an answer the refusal did not name. <!-- from: b0e865b4-9204-4c55-9f5b-dcb755b3a03b, 48f4647d-8c81-4e8a-a936-6b32f5fb0699, 98d5d389-8315-41da-9970-666e31f3a94d, 50b31713-4fea-4aa3-842a-79c2750660ba -->
Never pass `--no-verify`; never edit or remove the tool's hooks. <!-- from: 4f925b13-ad16-48b8-acd1-3e30d6e308b6, 292359a4-0512-465b-b6a7-79838e66e5a5, 98d5d389-8315-41da-9970-666e31f3a94d, 6ac1dde1-fe11-47af-861a-334dbf917b11 -->
The tool writes `adrdom/state/`; its tracked files are staged with the commit, never edited. Approve writes the `id:` and `terms:` lines, answer the `**Answer:**` line. <!-- from: bf6697e3-8832-4b90-b036-8fecedb55406, 06b7e003-4d9f-427a-a798-c1af97865411, ec3f875c-a0f8-4085-8da1-fec9dcb14377, 30ca5d13-e68b-465f-aa84-ea4000c9330a, 7a1e7b81-0967-4ed2-8a8a-c7153056836f, f571a454-3e6c-46c1-b754-0822a5d1d667, b36113a1-1384-4e4c-a798-8399f4181366, 3762e14b-6dfe-45ee-b407-d2718e817348 -->
Behaviour no current decision covers is a question for the owner before it is built. A decision never changes once answered: a later question supersedes it. A decision that looks wrong is said in chat, never edited or worked around. <!-- from: c3127af2-4c96-48d5-a164-bac49a9a09ec, dad5273c-afe2-4ab1-a451-e2a5d4d9a26d, 2bae531b-c4df-48a0-89a0-3d52726741ea, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, e2533580-1486-4dc9-8697-e6c53605fe72 -->
`adr ban` bans a word the owner does not want in the repository; only the owner unbans. <!-- from: ef7ec04d-9cdd-40a1-8246-42a4a0200828, 4f04bae4-98e3-48eb-9403-3257268e2d87, ef83beae-6857-4200-b251-a0fcc3c72f9d -->

### Build

Build a decision when a move names it, and not before; never one whose question this session wrote. <!-- from: 9ba58b6a-bc85-44a5-a720-d8e7a6ff7b31, f8d92841-aef7-4cd7-bd94-0e07c03e4cb6, 9a2ff21b-c20b-4c63-95f2-a7a3cbe236b8 -->
A decision's id is on the `id:` line under its heading; `adr show --ids` prints it. <!-- from: cdc905a2-5b58-4c38-beff-4a8bd3b6f42e, 7c0b9fe3-6038-4a23-93fe-88a31c7eaf40, 7a1e7b81-0967-4ed2-8a8a-c7153056836f, c5e10ae8-258d-4f29-b9fe-13cb7ad2a550 -->
A decision is built once a test outside `adrdom/` carries the comment `adr: <id>` and shows the code follows it; write the test if none exists. One test may carry several `adr:` comments. <!-- from: 4319a653-6901-446c-94ab-6420ef472134, 3460d220-e523-448b-aeff-baa8051b7d06, c49997ee-10e7-48e0-b66b-43f8b09758a6 -->
`adr check` reports each untested current decision until a test names it. A test whose `adr: <id>` names a decision that is not current is reported and fails nothing: change its `adr:` comment to the decision that took its place, or remove the comment. <!-- from: 3460d220-e523-448b-aeff-baa8051b7d06, 2381d759-f85f-484e-9031-fadbe6ea045a, 50b31713-4fea-4aa3-842a-79c2750660ba -->
Code that follows a decision may carry the comment `adr-code: <id>`, found by search; it counts as no test and no rule judges it. <!-- from: 1259323f-cdec-4101-9d47-a165a286089f -->
In a satellite, `adrdom/primary` holds the path to the record; a test there with `adr: <id>` counts for the primary, and a command that writes the record runs in the primary. <!-- from: a8280dd1-4bfa-4eeb-9656-1cd06381590a, 13ddec51-ce03-4645-99ae-2bbf4f30712d, 0e16fbf3-1401-4538-bd12-25303994af10 -->

### Review

A behaviour is what the code does or prints. In a review of a build, name the decision behind each behaviour the diff adds or changes; a behaviour with no decision behind it is a finding. Findings go in chat. <!-- from: 125ee772-d011-4dab-b53e-97a09ec03198, c3127af2-4c96-48d5-a164-bac49a9a09ec, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, 0a742360-83c2-479d-b858-4bea646c9b36 -->

## decisions

### Terms

Every word the owner gave a fixed meaning is a term, and in the records it means the term only. The terms brief holds one line per term; every agent reads it before it writes a question, code or a reply: <!-- from: 521e6211-344a-4b56-a1aa-07be956b3b29, f1294391-48b0-4ba3-82b4-c1d6b1c2c884, adee66f2-c8ff-477f-8ac4-a497f694c912, f590059d-b681-40ec-8a1c-c8916d71f6bb, 7d44030a-3589-4f29-aaa6-12fa09908ab8 -->

@terms-brief.md

Every question holds at least one term. A new word enters `adrdom/terms.md` under its `# Terms` line: <!-- from: 711a7636-9f4d-402d-ae62-265208adf0ac, 7c306235-6243-4d79-b9aa-08ed19662779, fcc1c1ec-d8e8-428e-9731-9817cdeb8e76, 528fe6f6-6d77-42d5-9086-017048d1448d -->

```
## <name>
definition: What the thing is, never what it does.
aliases: the other forms of the name, comma separated
```

A term's identifier carries its meaning: the name may change, and a former name goes to `aliases:`. A retired term, `retired: yes` in its entry, holds no words. A term is retired only once no question names it: the decisions naming it are superseded first. Terms are approved before any topic file that names them, one entry at a time: `owner runs: adr approve adrdom/terms.md <name>`. <!-- from: 7c306235-6243-4d79-b9aa-08ed19662779, 5577f646-a9d4-4785-9193-2317b9aa502d, c584a68d-e54a-4ad2-8a6b-9d64ed098ae5, fcf9d2c6-009e-4d6a-a3e4-14274bd1ee94, aa475aa3-05b2-429a-b33a-46b1dcb52049, da0dc8b1-58cd-4a16-8e64-234e4bfd5320, 528fe6f6-6d77-42d5-9086-017048d1448d -->

### Topic files

A topic file, `adrdom/decisions/<topic>.md`, holds the questions of one subject: a `# <Topic>` line, a few lines of prose, then the headings. <!-- from: 9913568f-6b85-4d26-946b-c6e108f9aa51, 625dea67-13a7-4e8e-913f-e997081a120c, 08193e75-dd89-4857-8e9f-b19e1a9e75e6 -->
A heading is a `###` line and one question: "Does X ..." or "What does X ...". One that opens with which, where, how many or how much assumes its answer; one with "and" or "or" is two questions. <!-- from: 9913568f-6b85-4d26-946b-c6e108f9aa51, 149c9a73-16eb-46aa-9806-2950423ee1a2, b3cb190d-d6f1-4930-bdcd-7827e31ac62a, 277a8338-c57a-43cc-8b28-6b42e9347f6a -->
Under the heading sit its lines, `depends:`, `supersedes:` and `blame:`, entries comma separated; then the body: <!-- from: a065dced-a3b3-467c-b299-17bb70742046, 9afe335c-2a7a-4625-b115-f74d3f091cac, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, 69708a07-469c-406b-9e25-63da1605b432, b0554224-15e4-4e91-a2bb-9ae5667803f4 -->

```
### Does the pre-commit hook run check?
depends: check, Does check read staged files only?=A

Problem: what breaks, in one line.
Standard: the named practice for this, stated flat.

- A) One sentence that answers the heading, twenty words at most.
  Why: what this gains, twenty words at most.
  Costs: what this loses, twenty words at most.
- B) ...
```

Two or three options. Under a heading that opens with Does, an option is one word with its full stop, `- A) Yes.`; the Why and Costs lines carry the rest. Two options that answer the heading the same way are one option. <!-- from: 178a232e-b51c-4122-a1a9-9e9ed6970748, b3cb190d-d6f1-4930-bdcd-7827e31ac62a, 43778946-d44c-4acf-a766-0a4915fce199 -->
A decision binds by its option sentence; Why and Costs explain it, and on a clash the option holds. <!-- from: 28b5837d-46ae-459e-ba4c-12265ef07ac1 -->
A question holds nothing of the day it was written: no word on what the code does now ("already", "currently", "as built", "unchanged"), who said what ("agreed", "decided", "in chat", an agent's name), or a date. <!-- from: 149c9a73-16eb-46aa-9806-2950423ee1a2 -->

### Plan

`adr plan <file>` does the step the file is at and prints the next move, until the move is `owner runs: adr approve <file>`. The steps: <!-- from: 46fc51ff-66b9-4841-a953-ae0d4c2fc424, d3be71e4-ce7b-43b0-86a8-e86868521f99 -->
1. Write the headings, then run plan: for each heading it brings the nearest current decisions and terms, its neighbours. <!-- from: 46fc51ff-66b9-4841-a953-ae0d4c2fc424, de8ef568-3a8d-42ba-bef7-d7fd467b27be, daec9ffd-676f-4111-a359-2827bfd4527c -->
2. Write a line for each neighbour the question stands on or replaces: `depends:` when the question loses its meaning without it or gains its meaning from it, `supersedes:` when the question replaces that decision. A neighbour on neither gets no line. <!-- from: a065dced-a3b3-467c-b299-17bb70742046, de8ef568-3a8d-42ba-bef7-d7fd467b27be, aef1b384-2b33-47a2-92a8-ec7f8fb258ef, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, f4f667ad-8714-4e65-897b-c7002e423c28 -->
3. Write the bodies. Before an option, read the code and the decisions it meets: what the code has to keep is the option's Costs. A second question found on the way is a new heading. <!-- from: f5eeaacf-1c9f-478b-8a97-2df1aaf39458, 178a232e-b51c-4122-a1a9-9e9ed6970748 -->
4. Run plan again: it brings the neighbours of each body, and those brought for its heading, with the reader's word, `same`, `consistent`, `inconsistent`, `unrelated` or `closes <letter>`, and a reason. The reader's word is not a ruling; the agent's line is. Run plan until the move it prints is the owner's. <!-- from: 46fc51ff-66b9-4841-a953-ae0d4c2fc424, d10dab89-1d42-4764-9d72-22d6fd6a9260, 55a6a2d8-03af-4db9-a4c7-e2790e38357e, e1cbe055-5638-41ea-853e-13991b945ae1, aae3f502-1395-4812-96ef-71510a1df4db, a065dced-a3b3-467c-b299-17bb70742046, e78b8297-fd89-4e09-9c3e-9eeb0fd0f654, be0d6f5c-ae1b-4753-8ec7-0f4b03d84641 -->
5. Say the plan is ready, post `owner runs: adr approve <file>` and stop. <!-- from: 46fc51ff-66b9-4841-a953-ae0d4c2fc424, 23d53545-c238-48cc-ae5e-21a2c7aab83e, aa275fac-def5-42a2-8a61-660142acad8e -->

A question stands on what it loses its meaning without or gains its meaning from: a term, or a question of the same file; a depends line names at most two questions. Before approval an entry on any line is the term's name or the heading of the question or decision; approve turns them into ids. An entry on an open question carries its answer set, `=A`, `=A|B` or `=*`: the letters under which the question still makes sense. <!-- from: 9afe335c-2a7a-4625-b115-f74d3f091cac, 010901e0-c02d-49eb-98bc-d584756d7f51, aef1b384-2b33-47a2-92a8-ec7f8fb258ef, 7c0b9fe3-6038-4a23-93fe-88a31c7eaf40, 946263aa-519a-47aa-b743-5cdcf52f5d35, 9d634996-475e-4ddc-8113-05c404ad9d5c -->
A question that replaces a current decision names it on `supersedes:`, with every current decision standing on it and those standing on them; each leaves force once the question is answered. <!-- from: 796a2544-43e5-43db-b5d3-8a4e737ec9c6, f4f667ad-8714-4e65-897b-c7002e423c28, 2bae531b-c4df-48a0-89a0-3d52726741ea -->
`closes <letter>` says the record leaves only that option of the question standing: the question is decided, strike the heading. Write the neighbour on `supersedes:` instead only when the question is there to change that decision, and name every decision standing on it too. Under `inconsistent`, write it on `supersedes:` or reword the question so the two agree. A line kept against the reader's word is said in chat, with why. <!-- from: d10dab89-1d42-4764-9d72-22d6fd6a9260, f4f667ad-8714-4e65-897b-c7002e423c28, 21ae8ae0-6884-4d3c-8464-024a8a5aae2d, 0a742360-83c2-479d-b858-4bea646c9b36 -->
At most four questions, answered or not, stand on one question, and at most four of a file stand on terms alone; a question that stands on a question of its file sits a tier above it. The question these stand on is the one each of them loses its meaning without or gains its meaning from. <!-- from: b30c29aa-f2b4-4792-9c9f-acc645a29d4e, f1b4c4cc-97bd-446f-87b4-df5f1b6f4da2, f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, 9afe335c-2a7a-4625-b115-f74d3f091cac, aef1b384-2b33-47a2-92a8-ec7f8fb258ef -->
The owner reads the whole plan at each approval; a plan changes as a whole, not line by line: when one heading is wrong, check every heading of the file for the same flaw and propose the changes together, once. To change an approved plan, edit the file, run plan to the owner's move, and say in chat what changed and why. <!-- from: 796a2544-43e5-43db-b5d3-8a4e737ec9c6, f5eeaacf-1c9f-478b-8a97-2df1aaf39458, 23d53545-c238-48cc-ae5e-21a2c7aab83e, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd -->

### Ask and answer

`adr approve <file>` asks every unanswered question of the file at once and prints one line per ledger row it writes; or it prints everything that stops it, each with its move. A question carries no lean; the agent's lean goes in chat. <!-- from: 1d7c758d-1fd5-4e0c-8559-517315b15aaa, 0a742360-83c2-479d-b858-4bea646c9b36, 8846f6d1-cd8c-4b87-b6f2-35bc8adf6763, b0e865b4-9204-4c55-9f5b-dcb755b3a03b, 3762e14b-6dfe-45ee-b407-d2718e817348 -->
After approve: commit, post the lean for each question, post the `owner runs: adr answer ...` lines `adr show` prints, and stop. The owner answers with a letter or `refuse`. <!-- from: 0a742360-83c2-479d-b858-4bea646c9b36, b782664f-7465-4e1e-99e0-13f6dd2de774, c3127af2-4c96-48d5-a164-bac49a9a09ec, 9ba58b6a-bc85-44a5-a720-d8e7a6ff7b31 -->
An answer outside a question's answer set removes that question and every question standing on it from the file. An answer strikes the section of every superseded question; the next commit carries those files. <!-- from: 462e22c6-8b23-4061-9a2a-bbf8691fce60, f5c242b6-e2f6-45a3-8c61-c743f3ad794f, 9d634996-475e-4ddc-8113-05c404ad9d5c, b5ca161f-c88c-4f02-a9d9-1d406cf3a252, dfd32ef6-c2ac-40ef-b8b3-9c58732516b1 -->
`adr show <file>` prints each question with its status: `[no body]`, `[not asked]`, `[asked @<version>]`, `[changed since asked]`, `[answered A]`, `[refused]`, `[superseded]`, `[withdrawn]`. <!-- from: b0acc95b-3dd5-4158-ac5c-65a2159972a9, 209081f4-864f-4ef6-8909-65da87f135e5, f53ac3b9-efde-427c-8027-19988aa8fda7 -->

### Blame

An old prose file, the origin, enters the records through a blame file, `adrdom/blame/<name>.md`, which the owner approves before any heading is written from it. Git keeps the text; the blame file copies none of it. <!-- from: e879fe72-ec07-48cd-9902-2b158aaae726, d2851ea1-fcb6-476a-b321-c813a594d5e5, ecb18413-cb3e-437e-9444-3cdd406ed67d, 108053a8-c2c3-4561-874c-2f573a5409f2 -->

```
origin: docs/adr/old-file.md
blob: <git hash-object docs/adr/old-file.md>

1-9	detail
10-14	decision
15-22	detail
```

A row is a line range, a tab, and one word: `decision` marks a range the owner settles by picking an option, `detail` every other range. A single line is the range `3-3`. The ranges reach every line of the origin, counted as an editor shows them; a range may appear in more than one row. Whether a decision is in the records, or contradicts one, is found in planning, not in the row. <!-- from: d2851ea1-fcb6-476a-b321-c813a594d5e5, 5c399a14-1e5c-48eb-b502-b57524c01c24, e879fe72-ec07-48cd-9902-2b158aaae726, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f -->
Approve writes an `id:` line under `blob:`. While the origin is in the repository, check compares it with `blob:`: when the origin changes, write the rows and `blob:` again, keep `id:`, and the owner approves again. Once approved, the owner may delete the origin. <!-- from: e879fe72-ec07-48cd-9902-2b158aaae726, d2851ea1-fcb6-476a-b321-c813a594d5e5, 6f6e7f4a-d733-4f3a-99f9-6a47fec5db70, 0c3d498a-c540-47f3-8384-050fc044c109 -->
Then write the headings from the decision rows, each in the topic file of its subject, with a blame line under the heading: `blame: old-file 10-14 23-30, other 3-9`: the name without `.md`, then the range of each row the question came from; a second blame file after a comma. A row that planning merges into another heading, or drops as a copy, needs no heading; `adr show --questions <blame file>` prints how many rows each question names and how many none does. From there the plan goes its steps. <!-- from: 69708a07-469c-406b-9e25-63da1605b432, e879fe72-ec07-48cd-9902-2b158aaae726, a469ecb9-a4f1-4a2a-89c5-7378cc1138b4, 46fc51ff-66b9-4841-a953-ae0d4c2fc424 -->
