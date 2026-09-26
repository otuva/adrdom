# Owner

What the owner's acts do beyond the answer itself, and the page the owner performs them on.

### Does an owner command commit?
depends: 1d7e59ea-99e3-4d4b-9928-5dabca62723d, b36113a1-1384-4e4c-a798-8399f4181366, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd
id: dfd32ef6-c2ac-40ef-b8b3-9c58732516b1
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, answers=b36113a1-1384-4e4c-a798-8399f4181366, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d

Problem: an owner act writes the answer line and a ledger event; either every act makes its own commit, or the act travels with the next commit someone makes.
Standard: the ledger event is the act of record and check compares the file with it at every commit; the commit only packages the change.

- A) No: an owner command writes the file and the ledger and commits nothing; the next commit carries them, and check at that commit verifies they match.
  Why: the ledger event shows who acted, and the answer travels with the code and the test built on it.
  Costs: an answer sits uncommitted until the next commit.
- B) Yes: every owner command makes one commit with a fixed message.
  Why: the act is in history at once.
  Costs: ten answers are ten commits, and each one is stopped by the hook until its test exists.
- C) The page commits and the terminal does not.
  Why: no agent sits next to the page.
  Costs: two behaviours for one act.

**Answer:** A

### Does the owner override a stop in the tool?
depends: 1d7e59ea-99e3-4d4b-9928-5dabca62723d, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de
id: 48f4647d-8c81-4e8a-a936-6b32f5fb0699
terms: answers=b36113a1-1384-4e4c-a798-8399f4181366, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: a gate that holds a rule nobody settled walls the owner in; a gate that holds a settled rule has a legal move, and a second door beside it skips the event that says what changed.
Standard: every stop message names the move that clears it; an escape hatch answers rules nobody settled.

- A) No: there is no override; every stop names the move that clears it and who makes it, and a case with no legal move is a new question.
  Why: every rule a gate holds is a decision, so a legal move always exists.
  Costs: a stop the owner disagrees with needs a new decision, not a one-off pass.
- B) Yes: an owner-only override names the stop and a reason, writes one event, and lets the next run pass once.
  Why: the owner is never held.
  Costs: a second door beside every legal move, and it skips the event that says what changed.
- C) Owner commands never stop the owner; they only warn.
  Why: the owner is never stopped.
  Costs: a warning the owner walks past on every guard.

**Answer:** A

### Does a question carry the agent's lean?
depends: 277a8338-c57a-43cc-8b28-6b42e9347f6a, f90dbd5b-8fbe-4899-8588-f8762795c2e2
id: 0a742360-83c2-479d-b858-4bea646c9b36
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, options=43778946-d44c-4acf-a766-0a4915fce199, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: options with no recommendation make the owner do the synthesis, but a recommendation written into the record sits beside the decision after the answer and reads as part of it.
Standard: decision records hold the options and the choice; the reviewers' discussion lives in the review thread.

- A) No: the lean lives in the conversation where the question is presented; the topic file, the ledger and the page hold none.
  Why: the tool holds only what the owner picked, never what an agent argued.
  Costs: the owner reads the page and the conversation side by side.
- B) Yes: a lean line in the question, outside the hash.
  Why: the recommendation sits next to the options.
  Costs: an agent can edit it after the ask, and it reads as part of the record.
- C) The agent writes the lean into the asked event, and the page shows it beside the question.
  Why: the page carries the recommendation.
  Costs: the ledger holds an agent's argument.

**Answer:** A

### Does the ui perform every owner act?
depends: 7127ab1e-c313-4c51-9134-c09d551682cf, 1d7e59ea-99e3-4d4b-9928-5dabca62723d
id: 644b43ce-be10-4e9a-a7d5-71b27c51e365
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, answers=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, current=e2533580-1486-4dc9-8697-e6c53605fe72, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, options=43778946-d44c-4acf-a766-0a4915fce199, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, topic=292a10b8-c287-4d71-b2bd-840da0a6de61, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6, ui=7127ab1e-c313-4c51-9134-c09d551682cf

Problem: when the page performs only some owner acts, the owner goes back to the terminal for the rest on every topic.
Standard: an owner surface covers every act only the owner may perform and shows the state those acts need.

- A) Yes: approve, answer and unban all run from the page, and the page shows every topic file's state, open questions with their options, current decisions and check results.
  Why: nothing the owner does needs a terminal.
  Costs: three forms to guard instead of one.
- B) No: the page answers only; approve and unban stay in the terminal.
  Why: a smaller surface to guard.
  Costs: the terminal for every topic.

**Answer:** A

### Does the adr binary serve the ui?
depends: 644b43ce-be10-4e9a-a7d5-71b27c51e365=*, 7127ab1e-c313-4c51-9134-c09d551682cf
id: ece60954-56d4-4622-883e-265f3b997c9f
terms: install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, ui=7127ab1e-c313-4c51-9134-c09d551682cf

Problem: the page needs a process that reads the repository and performs owner acts; if that process is a separate program, the page's rules can drift from the command line's.
Standard: developer tools serve their local page from the same binary as the command line, calling the same functions.

- A) Yes: adr serve serves a local page, and every act on it calls the same code as the command.
  Why: the page cannot differ from the command line on a rule, because it runs the same code.
  Costs: a web server inside the command-line tool.
- B) No: a separate app calls the adr binary.
  Why: the command-line tool stays small.
  Costs: two things to install and keep in step.

**Answer:** A

### What does the ui require before an owner act?
depends: 644b43ce-be10-4e9a-a7d5-71b27c51e365, aa275fac-def5-42a2-8a61-660142acad8e, 7127ab1e-c313-4c51-9134-c09d551682cf
id: df452a5a-1e2d-497e-93e9-7d6a80e8540d
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, ui=7127ab1e-c313-4c51-9134-c09d551682cf

Problem: a local server takes requests from any process on the machine, including an agent's, and the terminal an owner command looks for does not exist in a browser.
Standard: local admin pages bind to loopback, print a one-time token at start, swap it for an httponly SameSite=Strict cookie, and verify it on every act.

- A) A token: adr serve is an owner command, listens on 127.0.0.1 only, prints a fresh token once per start that the browser swaps for a cookie, and every act verifies the cookie; the token never enters the conversation.
  Why: the agent never holds the credential, and the owner-command gate guards starting the server.
  Costs: the owner opens one link per start.
- B) Nothing beyond listening on 127.0.0.1 only.
  Why: nothing to copy.
  Costs: any agent process on the machine can post an answer.
- C) A separate operating-system user or socket permissions.
  Why: the owner's identity is kept apart from the agent's.
  Costs: machine setup, beyond the threat the tool guards against.

**Answer:** A

### Does an answer carry the version of the question the owner read?
depends: 644b43ce-be10-4e9a-a7d5-71b27c51e365, b36113a1-1384-4e4c-a798-8399f4181366, 277a8338-c57a-43cc-8b28-6b42e9347f6a, 1d7e59ea-99e3-4d4b-9928-5dabca62723d
id: 11be073c-ce91-44d8-833a-ef0b5e0b1672
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, asks=3c8d4c9c-abca-4faa-b7c1-aae39951376c, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, refuses=0942e445-0163-423e-8ba7-84dd8ac07045, withdraws=e4a2ef0c-3354-4c9b-a8d9-9e204e544d85

Problem: an asked question may be edited and asked again while the owner reads it, and the answer then lands on text the owner never read.
Standard: optimistic concurrency control has a write carry the version it was based on and refuses it when that version is stale, as with an HTTP If-Match header or review votes bound to one patch set.

- A) Yes: show and the page print a short version of each asked question, answer takes that version from the terminal and the page alike, and a version that no longer matches refuses the answer.
  Why: an answer always lands on the text the owner read, and an asked question can still be fixed and asked again.
  Costs: the owner carries one more value with each answer.
- B) No: an asked question never changes, and to change it the agent withdraws it and asks a new question.
  Why: the text the owner reads cannot move.
  Costs: every fix to an asked question costs a plan approval and breaks the depends lines that name it.
- C) No: answer takes only the letter and goes to the text last asked.
  Why: nothing more to carry.
  Costs: an edit made while the owner reads puts the answer on text the owner never read.

**Answer:** A
