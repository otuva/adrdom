# adrdom

This repository is governed by `adr`. What the code does is a decision: the option the owner picked when they answered a question. <!-- from: c3127af2-4c96-48d5-a164-bac49a9a09ec, 125ee772-d011-4dab-b53e-97a09ec03198, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, b36113a1-1384-4e4c-a798-8399f4181366 -->
Agents write the questions. Only the owner approves a plan and answers a question, at their own terminal. <!-- from: 23d53545-c238-48cc-ae5e-21a2c7aab83e, aa275fac-def5-42a2-8a61-660142acad8e, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, b36113a1-1384-4e4c-a798-8399f4181366 -->
The git hooks run `adr check` at every commit and push; check refuses what breaks a decision. <!-- from: 34e81f57-9b3c-4dc4-a2ba-ca227523177d, 19e1c023-a678-4842-a414-bb0e45c41601, 4f925b13-ad16-48b8-acd1-3e30d6e308b6, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de -->

## agents

`adr show` prints every current decision, then the findings and the moves. <!-- from: c5e10ae8-258d-4f29-b9fe-13cb7ad2a550, 1af76f91-5644-42ef-97ea-8f70f934c26c -->
A move opens with who makes it: `agent`, `builder` (another session), `owner runs: <command>` (the owner, at their terminal), or `not yet`. <!-- from: b0e865b4-9204-4c55-9f5b-dcb755b3a03b, 9a2ff21b-c20b-4c63-95f2-a7a3cbe236b8, d904dc0d-231f-4e93-ac74-7babe888107d -->
A finding is what the tool found and the one move that clears it; an owner command clears nothing a finding did not name. <!-- from: 50b31713-4fea-4aa3-842a-79c2750660ba, b0e865b4-9204-4c55-9f5b-dcb755b3a03b, 48f4647d-8c81-4e8a-a936-6b32f5fb0699, ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862 -->
Behaviour no current decision covers is a question for the owner before it is built; a decision never changes once answered, a later question supersedes it. <!-- from: c3127af2-4c96-48d5-a164-bac49a9a09ec, dad5273c-afe2-4ab1-a451-e2a5d4d9a26d, 2bae531b-c4df-48a0-89a0-3d52726741ea -->
A session that runs `adr plan` or `adr ask` is a planner until it ends: it does not build the decisions it asked, and its commits hold only paths under `adrdom/`. A subagent runs under the session's identifier and carries its role. <!-- from: 6c406624-b874-4697-9a44-91941019b7d1, 1ee664a1-cac0-4917-9e46-8199a387d235, 9ba58b6a-bc85-44a5-a720-d8e7a6ff7b31, db07a54d-4d80-4035-a9ef-eabcc0c872b3, e53b805c-9272-4b89-985d-61426fb5fa48 -->
A builder is another session; it builds from the record alone, and a decision is built once a test outside `adrdom/` carries the comment `adr: <id>` and shows the code follows it. <!-- from: 9ba58b6a-bc85-44a5-a720-d8e7a6ff7b31, 4319a653-6901-446c-94ab-6420ef472134, 3460d220-e523-448b-aeff-baa8051b7d06, c49997ee-10e7-48e0-b66b-43f8b09758a6 -->
In a review of a build, a behaviour with no decision behind it is a finding. <!-- from: 125ee772-d011-4dab-b53e-97a09ec03198 -->
A decision binds by its option sentence; Why and Costs explain it, and on a clash the option holds. <!-- from: 28b5837d-46ae-459e-ba4c-12265ef07ac1 -->
Code that follows a decision may carry the comment `adr-code: <id>`, found by search; it ties no test and no rule judges it. <!-- from: 1259323f-cdec-4101-9d47-a165a286089f -->
The tool writes `adrdom/state/`; its tracked files are staged with the commit, never edited. Approve writes the `id:` lines, ask the `terms:` line, answer the `**Answer:**` line. <!-- from: bf6697e3-8832-4b90-b036-8fecedb55406, 06b7e003-4d9f-427a-a798-c1af97865411, ec3f875c-a0f8-4085-8da1-fec9dcb14377, 30ca5d13-e68b-465f-aa84-ea4000c9330a, 7a1e7b81-0967-4ed2-8a8a-c7153056836f, f571a454-3e6c-46c1-b754-0822a5d1d667, b36113a1-1384-4e4c-a798-8399f4181366 -->
`adr ban` bans a word the owner does not want in the repository; only the owner unbans. <!-- from: ef7ec04d-9cdd-40a1-8246-42a4a0200828, 4f04bae4-98e3-48eb-9403-3257268e2d87, ef83beae-6857-4200-b251-a0fcc3c72f9d -->

## decisions

Every word the owner gave a fixed meaning is a term, and in the records it means the term only. The terms brief holds one line per term; every agent reads it before it writes a question, code or a reply: <!-- from: 521e6211-344a-4b56-a1aa-07be956b3b29, f1294391-48b0-4ba3-82b4-c1d6b1c2c884, adee66f2-c8ff-477f-8ac4-a497f694c912, f590059d-b681-40ec-8a1c-c8916d71f6bb, 7d44030a-3589-4f29-aaa6-12fa09908ab8 -->

@terms-brief.md

Every question holds at least one term. A new word enters `adrdom/terms.md`, as `## <name>`, a `definition:` line and an `aliases:` line; a definition says what the thing is, never what it does. Terms are approved before any topic file that names them. <!-- from: 711a7636-9f4d-402d-ae62-265208adf0ac, 7c306235-6243-4d79-b9aa-08ed19662779, fcc1c1ec-d8e8-428e-9731-9817cdeb8e76, aa475aa3-05b2-429a-b33a-46b1dcb52049, da0dc8b1-58cd-4a16-8e64-234e4bfd5320, 528fe6f6-6d77-42d5-9086-017048d1448d -->
A topic file, `adrdom/decisions/<topic>.md`, holds the questions of one subject: a `# <Topic>` line, a few lines of prose, then the headings. <!-- from: 9913568f-6b85-4d26-946b-c6e108f9aa51, 625dea67-13a7-4e8e-913f-e997081a120c, 08193e75-dd89-4857-8e9f-b19e1a9e75e6 -->
A heading is one question: "Does X ..." or "What does X ...". One that opens with which, where, how many or how much assumes its answer; one with "and" or "or" is two questions. <!-- from: 149c9a73-16eb-46aa-9806-2950423ee1a2, b3cb190d-d6f1-4930-bdcd-7827e31ac62a, 277a8338-c57a-43cc-8b28-6b42e9347f6a -->
The body: <!-- from: 178a232e-b51c-4122-a1a9-9e9ed6970748, f5eeaacf-1c9f-478b-8a97-2df1aaf39458 -->

```
Problem: what breaks, in one line.
Standard: the named practice for this, stated flat.

- A) One sentence that answers the heading, twenty words at most.
  Why: what this gains, twenty words at most.
  Costs: what this loses, twenty words at most.
- B) ...
```

Under a heading that opens with Does, an option is one word. Two options that answer the heading the same way are one option. <!-- from: b3cb190d-d6f1-4930-bdcd-7827e31ac62a, 43778946-d44c-4acf-a766-0a4915fce199 -->
A question holds nothing that ties it to the day it was written: no word on what the code does now, who said what, or a date. <!-- from: 149c9a73-16eb-46aa-9806-2950423ee1a2 -->
Under each heading, every neighbour `adr plan` brings goes on one line: `depends:`, `supersedes:` or `unrelated:`. <!-- from: c6451ecc-17a9-447e-a9c9-fe9b35ec789c, de8ef568-3a8d-42ba-bef7-d7fd467b27be, daec9ffd-676f-4111-a359-2827bfd4527c -->
A question stands on what it loses its meaning without: a term, or a question of the same file; a depends line names at most two questions. An entry on an open question carries its answer set, `=A`, `=A|B` or `=*`: the letters under which the question still makes sense. <!-- from: 9afe335c-2a7a-4625-b115-f74d3f091cac, 010901e0-c02d-49eb-98bc-d584756d7f51, 946263aa-519a-47aa-b743-5cdcf52f5d35, aef1b384-2b33-47a2-92a8-ec7f8fb258ef, 9d634996-475e-4ddc-8113-05c404ad9d5c -->
A question that replaces a current decision names it on `supersedes:`, with every current decision standing on it; each leaves force once the question is answered. <!-- from: 796a2544-43e5-43db-b5d3-8a4e737ec9c6, f4f667ad-8714-4e65-897b-c7002e423c28, 2bae531b-c4df-48a0-89a0-3d52726741ea -->
A tier holds at most four questions; tiers are unlimited: a question that stands on a question of its file sits a tier above it. The question a tier stands on is the one each of its questions loses its meaning without. <!-- from: 0e38f001-0226-4927-9a78-2e9fc1ec18d4, f1b4c4cc-97bd-446f-87b4-df5f1b6f4da2, f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, 9afe335c-2a7a-4625-b115-f74d3f091cac, aef1b384-2b33-47a2-92a8-ec7f8fb258ef -->
`adr plan <file>` does the step the file is at and prints the next move, until the move is `owner runs: adr approve <file>`. <!-- from: 46fc51ff-66b9-4841-a953-ae0d4c2fc424, d3be71e4-ce7b-43b0-86a8-e86868521f99 -->
The owner reads the whole plan at each approval, so a plan changes as a whole, not line by line. <!-- from: 796a2544-43e5-43db-b5d3-8a4e737ec9c6, f5eeaacf-1c9f-478b-8a97-2df1aaf39458, 23d53545-c238-48cc-ae5e-21a2c7aab83e, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd -->
`adr ask <file>` asks every unanswered question of the file at once; a question carries no lean, the agent's lean goes in chat. <!-- from: 1d7c758d-1fd5-4e0c-8559-517315b15aaa, 0a742360-83c2-479d-b858-4bea646c9b36 -->
An old prose file enters the records through a blame file under `adrdom/blame/`, which the owner approves before any heading is written from it. <!-- from: e879fe72-ec07-48cd-9902-2b158aaae726, d2851ea1-fcb6-476a-b321-c813a594d5e5, ecb18413-cb3e-437e-9444-3cdd406ed67d, 108053a8-c2c3-4561-874c-2f573a5409f2 -->
A blame file holds `origin: <path>`, `blob: <hash>`, a blank line, then one row per line range, `<start>-<end>`, a tab, `decision` or `detail`, over every line of the origin; a question written from it carries `blame: <file> <start>-<end>`. <!-- from: d2851ea1-fcb6-476a-b321-c813a594d5e5, 5c399a14-1e5c-48eb-b502-b57524c01c24, 69708a07-469c-406b-9e25-63da1605b432 -->
