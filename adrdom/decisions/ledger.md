# Ledger

What one ledger row holds, what identifies a question, and what keeps the ledger append-only.

### What does a ledger row hold?
depends: e18b82d8-5271-4385-bbab-2b89e5da278f
id: f3e64c4e-21ab-442f-8c40-6060591e2b77
terms: approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: the ledger is the record of every owner act and every ask; a row that cannot say when, what, which question and which bytes proves nothing later.
Standard: append-only audit logs write one line per event with a timestamp, an event type, the subject and a digest of the content.

- A) One tab-separated row per event in one append-only file: the time, the event, the identifier, a hash of the text it covers, a detail, and the tool version.
  Why: one line per event diffs cleanly in git and reads with any text tool.
  Costs: rows from two branches can conflict at the end of the file.
- B) One JSON object per line with the same fields.
  Why: named fields and room to grow.
  Costs: noisier diffs and a parser for a flat record.
- C) One file per question.
  Why: no conflicts at the end of one shared file.
  Costs: an event that spans questions, such as an approve, has no single home.

**Answer:** A

### Does a question keep its identifier when its heading is reworded?
depends: 277a8338-c57a-43cc-8b28-6b42e9347f6a, e877e4a9-2caf-4fe6-9f82-5476a2abb00b
id: 7a1e7b81-0967-4ed2-8a8a-c7153056836f
terms: identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: a question known by a slug or by its heading loses its identity on every rename, and every reference to it breaks.
Standard: stable keys; a record's identity is a value assigned once, apart from its title and location.

- A) Yes: the identifier is assigned once, never reused, and independent of the file, the heading and the wording; the text is hashed separately, so an edit is still caught.
  Why: identity survives renames and moves, and the hash still catches an edit.
  Costs: the identifier is unreadable in conversation, so every mention needs the heading beside it.
- B) No: the owner picks a slug for each question, and a rename is an owner act.
  Why: readable names.
  Costs: a rename is a change of state, and a slug invites its meaning to drift.

**Answer:** A

### Does every ledger row carry the tool version?
depends: e18b82d8-5271-4385-bbab-2b89e5da278f
id: 99619e27-19b6-47ef-83b9-c79ac30ee2f0
terms: ledger=e18b82d8-5271-4385-bbab-2b89e5da278f

Problem: when the tool's rules change between releases, a row cannot say which rules were in force when it was written.
Standard: records written by versioned software carry the writer's version, taken from one source of truth.

- A) Yes: a VERSION file at the repository root is the one source of the version; the binary embeds it, adr --version prints it, and every ledger row carries it.
  Why: any row says which release wrote it, which a later migration needs.
  Costs: one more field on every row.
- B) No: the version lives only in the binary.
  Why: shorter rows.
  Costs: a row cannot say which release wrote it.

**Answer:** A

### Does check fail a ledger row that was edited?
depends: e18b82d8-5271-4385-bbab-2b89e5da278f, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de
id: bf6697e3-8832-4b90-b036-8fecedb55406
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, agents=f90dbd5b-8fbe-4899-8588-f8762795c2e2, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d

Problem: the ledger is a text file an agent can edit; an edited row rewrites history the owner made.
Standard: append-only logs are verified against their last committed state; anything but new lines at the end is tampering.

- A) Yes: every row present at HEAD must be present, byte for byte, in the working copy; only new rows pass.
  Why: an edit or a deletion of a past event fails the next commit.
  Costs: a hand-forged new row still passes; only the deny rules keep agents from writing it.
- B) No: git history shows any edit, and review catches it.
  Why: nothing new in the tool.
  Costs: history the owner made can be rewritten in one commit.

**Answer:** A

### Does the tool take the order of ledger rows from their time?
depends: e18b82d8-5271-4385-bbab-2b89e5da278f, f3e64c4e-21ab-442f-8c40-6060591e2b77=A|B
id: 385a57d5-5f76-454a-ae5d-9b0d6f5c1492
terms: depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f

Problem: a merge joins the ledger rows of two branches in an order the merge sets, and the tool reads the last row in the file as the latest.
Standard: logs merged from several writers order their events by the time each event carries, not by where the event sits in the file.

- A) Yes: the latest row is the one with the latest time, and the place of a row in the file carries no meaning.
  Why: the result is the same whichever way two branches are merged.
  Costs: the result rests on the clock of the machine that wrote the row.
- B) No: the latest row is the last one in the file, and the time is a record only.
  Why: nothing rests on a clock.
  Costs: the result of a merge depends on which branch has its rows last.

**Answer:** A

### Does check at a merge look for the ledger rows of both branches?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, e18b82d8-5271-4385-bbab-2b89e5da278f, bf6697e3-8832-4b90-b036-8fecedb55406=A
id: 1c27f2d6-ab62-4c5a-91b2-1488722f3c48
terms: check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d

Problem: check compares the ledger with one side of a merge only, so a row of the other branch can be dropped while the merge is resolved and nothing reports it.
Standard: a merge of append-only logs is verified against every parent; the entries of each parent must all be present in the result.

- A) Yes: at a merge commit check fails unless every ledger row of each of the two branches is in the merged ledger.
  Why: no row is lost in a merge, whoever resolves it.
  Costs: check reads a second commit at every merge.
- B) No: check compares the ledger with the branch the merge runs on only.
  Why: check has one rule fewer.
  Costs: a row the owner wrote on the other branch can vanish at a merge with no failure.

**Answer:** A

### Does a command write a time later than the last ledger row?
depends: e18b82d8-5271-4385-bbab-2b89e5da278f, 385a57d5-5f76-454a-ae5d-9b0d6f5c1492=A
id: aefa7f3c-70c3-412e-8d73-16ff9e2b703a
terms: ledger=e18b82d8-5271-4385-bbab-2b89e5da278f

Problem: a machine whose clock runs behind writes a new ledger row that reads as older than the row before it, and two rows on one subject with the same time have no order.
Standard: logs ordered by time keep the time rising; a writer takes the later of its clock and one step after the last time in the log.

- A) Yes: a command writes its rows with one time, the later of the clock and one second after the last row of the ledger.
  Why: a new row is the latest whatever the clock says.
  Costs: the time of a row can run some seconds ahead of the clock.
- B) No: a command writes the time of the clock.
  Why: the time of a row is the time of the act.
  Costs: on a clock that runs behind, a new act reads as older than the one it replaces.

**Answer:** A

### Does check fail a ledger whose rows changed place?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, e18b82d8-5271-4385-bbab-2b89e5da278f, 385a57d5-5f76-454a-ae5d-9b0d6f5c1492=*
id: c10c3a49-e993-4c9f-a46b-4a1ba3646dcf
terms: asks=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, refused=0942e445-0163-423e-8ba7-84dd8ac07045, refuses=0942e445-0163-423e-8ba7-84dd8ac07045

Problem: a merge puts the ledger rows of one branch between the rows of the other, and a check that asks for the old ledger at the start of the file refuses the merge though no row is lost.
Standard: a log whose order comes from its entries is verified as a set: every old entry present, byte for byte, in any place.

- A) No: check fails a row that is missing or changed, and the place of a row in the file is free.
  Why: a merge passes whichever way it runs and however it is resolved.
  Costs: a row can be moved in the file with no failure; the move changes nothing the tool reads.
- B) Yes: the rows of the last commit keep their order among themselves, and new rows may stand between them.
  Why: a moved row is reported.
  Costs: a merge resolved with the rows of one branch out of their order is refused.
- C) Yes: the ledger of the last commit stands at the start of the file, byte for byte.
  Why: any change before the end of the file is reported.
  Costs: a branch that took the main branch in cannot be merged back into it.

**Answer:** A

### Does install write a merge rule for the ledger?
depends: 1b98cdea-dc1b-4789-8f67-68d130d7a49d, e18b82d8-5271-4385-bbab-2b89e5da278f, 385a57d5-5f76-454a-ae5d-9b0d6f5c1492=A
id: db56a285-946f-48b1-b073-bb1e90192185
terms: ban=ef83beae-6857-4200-b251-a0fcc3c72f9d, install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f

Problem: two branches that each add ledger rows conflict at every merge, and resolving the conflict is a hand edit of a file under adrdom/state.
Standard: for a file that only grows, git has a union merge, set in a git attributes file, that keeps the lines of both sides.

- A) Yes: install writes a git attributes file under adrdom that keeps the ledger rows of both branches at a merge; the ban list has no such rule.
  Why: a merge on the machine needs no hand edit of the ledger.
  Costs: hosting sites ignore the rule, and a merge made there still conflicts.
- B) No: the conflict stays, and whoever merges keeps the rows of both sides by hand.
  Why: install writes one file fewer.
  Costs: every merge of two branches with new rows needs a hand edit of the ledger.

**Answer:** A

### Does check hold the merge rule install writes for the ledger?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, 1b98cdea-dc1b-4789-8f67-68d130d7a49d, db56a285-946f-48b1-b073-bb1e90192185=A
id: 76c6adc6-c018-4df4-b49a-ddcd0a03a48c
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f

Problem: an agent can edit or remove the git attributes file, and a merge then conflicts again or keeps lines by a rule the tool never wrote.
Standard: generated files are compared with their generator, and drift fails.

- A) Yes: check fails when the git attributes file under adrdom differs from what install writes; the move is running install again.
  Why: the rule a merge runs by stays the text of the tool.
  Costs: a repository that adds its own lines to that file fails every commit.
- B) No: install writes the file once and nothing looks again.
  Why: the repository may add its own rules to the file.
  Costs: the rule can vanish, and nothing reports it until a merge conflicts.

**Answer:** A

### Does check look at the ledger before a push?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, e18b82d8-5271-4385-bbab-2b89e5da278f, bf6697e3-8832-4b90-b036-8fecedb55406=A
id: 19e1c023-a678-4842-a414-bb0e45c41601
terms: check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f

Problem: a rebase and a fast-forward make no commit, so no hook runs, and a ledger row lost that way is compared with nothing.
Standard: git runs a hook before a push with the commits of both sides; a check that a commit can miss is repeated there.

- A) Yes: install writes a hook that runs before a push, and check fails the push unless every ledger row of the remote branch is in the ledger that is pushed.
  Why: a row lost in a rebase or a fast-forward is caught before it leaves the machine.
  Costs: nothing is looked at until a push, and a push made without the hooks skips it.
- B) No: check looks at the ledger at a commit and at a merge only.
  Why: install writes one hook fewer.
  Costs: a row lost in a rebase or a fast-forward is reported nowhere.

**Answer:** A

### Does check in CI compare the ledger with the commit before the push?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, e18b82d8-5271-4385-bbab-2b89e5da278f, bf6697e3-8832-4b90-b036-8fecedb55406=A
id: a26876ba-5789-4d05-965e-853d00d9edd2
terms: check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f

Problem: check in CI reads the pushed commit alone, so a push that lost ledger rows passes.
Standard: a check on the server compares the pushed commit with the commit the branch held before the push.

- A) Yes: in CI check fails unless every ledger row of the commit before the push is in the ledger of the pushed commit.
  Why: a hook on the machine can be skipped, a check on the server cannot.
  Costs: the loss is reported after it reached the server, and the workflow fetches the earlier commit.
- B) No: check in CI reads the pushed commit alone.
  Why: the workflow needs one commit.
  Costs: a push that lost ledger rows passes CI.

**Answer:** A

### Does check fail a question answered with two different options?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, 277a8338-c57a-43cc-8b28-6b42e9347f6a, 43778946-d44c-4acf-a766-0a4915fce199, 1c27f2d6-ab62-4c5a-91b2-1488722f3c48=A
id: e3200081-9995-4bbe-983f-2f430dc341e7
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, answered=b36113a1-1384-4e4c-a798-8399f4181366, answers=b36113a1-1384-4e4c-a798-8399f4181366, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, option=43778946-d44c-4acf-a766-0a4915fce199, options=43778946-d44c-4acf-a766-0a4915fce199, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: two branches answer one question with different options; the merge keeps both answers in the ledger, whoever resolves the topic file picks the option, and check passes.
Standard: two writes to one record that disagree are not settled by the merge; they are reported, and the one with the authority writes again.

- A) Yes: check fails a question whose answers in the ledger name two different options, answer takes a new answer for it, and that answer stands.
  Why: the option that stands is one the owner picked with both in view.
  Costs: the owner answers one question twice.
- B) No: the option in the topic file stands, and the other answer stays in the ledger with no report.
  Why: nothing waits for the owner.
  Costs: whoever resolved the merge picked the decision.

**Answer:** A

### Does check fail two ledger rows on one subject with the same time?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, e18b82d8-5271-4385-bbab-2b89e5da278f, aefa7f3c-70c3-412e-8d73-16ff9e2b703a=A
id: ce19c883-c61d-477b-a6d4-fee5134c6e9a
terms: ban=ef83beae-6857-4200-b251-a0fcc3c72f9d, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f

Problem: two branches each write a ledger row on one subject in the same second, such as a ban and the lifting of that ban; both rows carry one time, and the time cannot say which is the later.
Standard: two writes to one record with the same time are a collision; they are reported and one of them is written again, not ordered by a rule that guesses.

- A) Yes: check fails two rows that carry one time and compete for one subject, and the command run again writes a later row that stands; rows of one command that share a time do not compete.
  Why: the row that stands is always the later one by the clock.
  Costs: a merge of two branches that wrote in the same second waits for one command to be run again.
- B) No: rows with one time are ordered by their text.
  Why: nothing waits, and the order is the same whichever way the merge ran.
  Costs: the row that stands is picked by its wording, not by which act came later.

**Answer:** A
