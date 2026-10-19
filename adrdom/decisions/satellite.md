# Satellite

One record may govern more than one repository: the primary holds the record, a satellite holds code and tests and no record of its own. These questions settle what install, check, show and uninstall do in each.

### Does install take the path of a satellite?
depends: install, satellite, primary
unrelated: Does install put the embedding model outside the repository?, Does install fetch the embedding model?, Does a command compute a move of its own?, Does check hold the files install writes?, Does check look at the ledger before a push?, Does install write outside adrdom?, Does uninstall belong to the owner?, Does uninstall remove a file install created?, What does the reader run?, What does check in the primary report for a satellite missing at its path?, What does install write in a satellite?, What does the primary hold of a satellite?, Does an owner command commit?, Does check hold the merge rule install writes for the ledger?, Does every ledger row carry the tool version?, Does install write a merge rule for the ledger?, What does install write for one agent?, Does a command that writes the record run in a satellite?, Does check hold a satellite's brief to the primary's?, Does check in a satellite report an untested decision?, Does check in the primary count a tie in a satellite?, Does show in a satellite name its primary?, Does uninstall in a satellite drop its row from the primary?, What does check in a satellite read the record from?

Problem: nothing makes a repository a satellite of a record, so a second repository has to carry a record of its own.
Standard: a hub-and-spoke tool learns its spokes at the hub, once, by one command.

- A) Yes.
  Why: one command in the primary names the satellite, writes both sides and leaves nothing to type by hand.
  Costs: the satellite's checkout has to be on disk when the command runs.
- B) No.
  Why: install keeps one shape; a satellite is made by hand.
  Costs: two files written by hand that have to agree, with nothing to say when they do not.

### What does check in a satellite read the record from?
depends: check, record, satellite, primary
unrelated: Does check at a commit read the staged files?, What does check do when an origin is absent?, Does check read the words of an answered question again?, move, Does check print a line with no move on it?, Does the tool read each current decision against its neighbours?, What does check do with an untested current decision?, Does check fail a ledger whose rows changed place?, Does a command refuse what check lets pass?, What does check in the primary report for a satellite missing at its path?, What does install write in a satellite?, What does the primary hold of a satellite?, Does check fail a ledger row that was edited?, Does check fail two ledger rows on one subject with the same time?, Does check look at the ledger before a push?, What does a command print when it writes nothing?, What does check do with a fault on a file awaiting approval?, Does a command that writes the record run in a satellite?, Does check hold a satellite's brief to the primary's?, Does check in a satellite report an untested decision?, Does check in the primary count a tie in a satellite?, Does install take the path of a satellite?, Does show in a satellite name its primary?, Does uninstall in a satellite drop its row from the primary?

Problem: a satellite has no ledger, no terms file and no topic file; check in it has no record to judge against.
Standard: a tool with a remote configuration reads it from the one place named, and fails loudly when it is missing.

- A) The primary's working tree, through the path the satellite holds; no record there and check stops.
  Why: one reading of the record for every command, the one the owner sees on disk; nothing is guessed.
  Costs: the primary's checkout has to sit at the path on every machine, CI included.
- B) The primary's last commit, through the path the satellite holds; no record there and check stops.
  Why: a satellite is judged only against a record that is in the primary's history.
  Costs: a ban or an answer not yet committed in the primary binds nothing in a satellite.
- C) A copy of the record install writes into the satellite.
  Why: a satellite stands alone, with no checkout beside it.
  Costs: every answer in the primary waits for a copy, and two records drift.

### What does the primary hold of a satellite?
depends: primary, satellite
unrelated: status, owner, What does a term hold?, Does a command compute a move of its own?, Does a depends entry on a question in its file carry an answer set?, Does a command name a status of its own?, Does a finding hold the identifier of the decision it judges?, Does a reference line keep its list on one line?, What holds the list of loaded phrases?, What does a depends entry name?, What does check in a satellite read the record from?, What does check in the primary report for a satellite missing at its path?, What does install write in a satellite?, Does check fail two ledger rows on one subject with the same time?, Does check hold the merge rule install writes for the ledger?, Does every ledger row carry the tool version?, Does the tool keep everything it owns under one directory?, What does a finding line hold?, What does a test carry to name its decision?, What does check do with a decision row no question names?, What does the tool keep of a reader's word?, Does a command that writes the record run in a satellite?, Does check hold a satellite's brief to the primary's?, Does check in a satellite report an untested decision?, Does check in the primary count a tie in a satellite?, Does install take the path of a satellite?, Does show in a satellite name its primary?, Does uninstall in a satellite drop its row from the primary?

Problem: the primary cannot find the tests of a satellite it does not know.
Standard: a hub keeps the list of its spokes beside its own state, one row each.

- A) One tracked row under adrdom/state: the satellite's relative path and its remote url.
  Why: check finds the satellite by the path, and CI can clone it by the url.
  Costs: a satellite that moves or changes remote needs its row written again.
- B) One tracked row under adrdom/state: the satellite's relative path.
  Why: the tool records only what it reads itself.
  Costs: CI learns where each satellite comes from outside the record.
- C) Nothing; CI names the satellites.
  Why: the smallest tool.
  Costs: the primary's check cannot count a satellite's ties.

### Does a command that writes the record run in a satellite?
depends: satellite, record, What does check in a satellite read the record from?=*
unrelated: Does a command compute a move of its own?, Does a command write a time later than the last ledger row?, Does a command name a status of its own?, What does a command print when it writes nothing?, What does a writing command print?, Does the ui render what the commands print?, Does a reader's word stop a command?, Does a command refuse what check lets pass?, Does an owner command commit?, Does a command stop when the environment gives no session identifier?, What does check in the primary report for a satellite missing at its path?, What does install write in a satellite?, What does the primary hold of a satellite?, Does a refusal name the command that clears it?, Does ban refuse a word that appears in the repository?, Does the tool rewrite the ban list from the ledger?, Does the tool write the ban list in one fixed order?, Does check hold a satellite's brief to the primary's?, Does check in a satellite report an untested decision?, Does check in the primary count a tie in a satellite?, Does install take the path of a satellite?, Does show in a satellite name its primary?, Does uninstall in a satellite drop its row from the primary?

Problem: plan, ask, approve, answer, ban, unban and serve write the record; in a satellite the record is in another repository.
Standard: a command that would write another repository's state refuses and names where to run.

- A) No.
  Why: the record is written in one place; the refusal names the primary's path.
  Costs: the owner and the planner change directory for every writing command.
- B) Yes.
  Why: the command writes through to the primary from any directory.
  Costs: a commit in the primary made from a satellite, by a hook that runs in the wrong repository.

### What does install write in a satellite?
depends: install, satellite, Does install take the path of a satellite?=A
unrelated: What does install write for one agent?, Does check hold the files install writes?, Does uninstall remove a file install created?, What does the reader run?, Does install write a merge rule for the ledger?, Does install put the embedding model outside the repository?, Does uninstall belong to the owner?, Does check look at the ledger before a push?, Does install fetch the embedding model?, Does check hold the merge rule install writes for the ledger?, What does check in the primary report for a satellite missing at its path?, What does the primary hold of a satellite?, Does install write outside adrdom?, Does the agent build what no current decision covers?, Does the agent write the plan?, Does the tool write a terms brief for the agent?, Does a command that writes the record run in a satellite?, Does check hold a satellite's brief to the primary's?, Does check in a satellite report an untested decision?, Does check in the primary count a tie in a satellite?, Does show in a satellite name its primary?, Does uninstall in a satellite drop its row from the primary?, What does check in a satellite read the record from?

Problem: an agent opening a satellite finds no brief and no hook, and writes code no decision covers.
Standard: every repository an agent works in carries the same brief and the same hooks.

- A) The primary's path under adrdom, copies of the brief and terms brief, the hooks and the AGENTS.md section.
  Why: the agent reads the same brief everywhere; the hook runs check at every commit.
  Costs: two copies to keep equal to the primary's.
- B) The primary's path under adrdom and the hooks; the brief is read in the primary.
  Why: nothing to keep equal.
  Costs: an agent that opens the satellite alone reads no brief.

### Does check in the primary count a tie in a satellite?
depends: primary, satellite, What does the primary hold of a satellite?=A|B
unrelated: What does check do with an untested current decision?, Does check fail a planner's commit that reaches outside adrdom?, Does a command name a status of its own?, Does a finding hold the identifier of the decision it judges?, Does a command refuse what check lets pass?, Does the tool read each current decision against its neighbours?, Does check in CI compare the ledger with the commit before the push?, Does check look at the ledger before a push?, Does check fail two ledger rows on one subject with the same time?, Does a command compute a move of its own?, Does the tier cap count a decision an open question supersedes?, What does check in the primary report for a satellite missing at its path?, What does install write in a satellite?, Does check at a commit read the staged files?, Does check print a line with no move on it?, What does check do when an origin is absent?, What does check do with a decision row no question names?, What does check do with a fault on a file awaiting approval?, What does check do with a test that names a decision that is not current?, Does a command that writes the record run in a satellite?, Does check hold a satellite's brief to the primary's?, Does check in a satellite report an untested decision?, Does install take the path of a satellite?, Does show in a satellite name its primary?, Does uninstall in a satellite drop its row from the primary?, What does check in a satellite read the record from?

Problem: a decision built in a satellite stays untested in the primary's eyes.
Standard: a traceability tool counts a tag wherever it scans, and scans every tree on its list.

- A) Yes.
  Why: one coverage for the whole record, and the build move names the satellite's path.
  Costs: every satellite's checkout at its path, or the count is short.
- B) No.
  Why: check reads one repository.
  Costs: a decision built in a satellite is untested for ever.

### Does show in a satellite name its primary?
depends: primary, satellite, What does check in a satellite read the record from?=A|B
unrelated: What does show print when no file is named?, Does a command name a status of its own?, Does show print the reference lines of a question that is answered?, What does a depends entry name?, Does a supersedes line name every current decision standing on its target?, Does a refusal name the command that clears it?, Does a finding hold the identifier of the decision it judges?, Does the ui render what the commands print?, Does a command compute a move of its own?, supersede, Does one decision supersede several?, What does check in the primary report for a satellite missing at its path?, What does the primary hold of a satellite?, What does install write in a satellite?, Does an answer carry the version of the question the owner read?, Does check hold the terms brief to the terms file?, Does the agent build what no current decision covers?, Does the brief name the terms brief?, Does the tool write a terms brief for the agent?, What does a reference in a topic file carry?, What does the tool print for a build move in a planner session?, Does a command that writes the record run in a satellite?, Does check hold a satellite's brief to the primary's?, Does check in a satellite report an untested decision?, Does check in the primary count a tie in a satellite?, Does install take the path of a satellite?, Does uninstall in a satellite drop its row from the primary?, What does check in a satellite read the record from?

Problem: an agent in a satellite sees decisions and cannot tell where they are written.
Standard: the tool says what the repository is; the brief says only how to work.

- A) Yes.
  Why: the first line of show says where the record is; the brief stays the same in every repository.
  Costs: one line more on every show in a satellite.
- B) No.
  Why: show prints the same in every repository.
  Costs: the brief has to carry a repository-specific line, or the agent asks.

### Does check in a satellite report an untested decision?
depends: satellite, decision, Does check in the primary count a tie in a satellite?=A
unrelated: What does check do with an untested current decision?, What does check do with a test that names a decision that is not current?, What does check do when an origin is absent?, Does a command refuse what check lets pass?, Does check fail a planner's commit that reaches outside adrdom?, Does check print a line with no move on it?, Does the tool read a pair of decisions in each direction?, What does check do with a decision row no question names?, Does check fail a topic file awaiting approval?, Does the tool read each current decision against its neighbours?, What does check in the primary report for a satellite missing at its path?, What does install write in a satellite?, What does the primary hold of a satellite?, Does a supersedes line name every current decision standing on its target?, Does check at a commit read the staged files?, Does check at a merge look for the ledger rows of both branches?, Does the agent build on a decision before its tier is fully answered?, Does the agent build what no current decision covers?, What does a test carry to name its decision?, What does the tool print for a build move in a planner session?, Does a command that writes the record run in a satellite?, Does check hold a satellite's brief to the primary's?, Does install take the path of a satellite?, Does show in a satellite name its primary?, Does uninstall in a satellite drop its row from the primary?, What does check in a satellite read the record from?

Problem: a satellite sees its own tests only; a decision tested in another repository looks untested there.
Standard: a coverage report is made where every tree is visible, and nowhere else.

- A) No.
  Why: the build moves come from the primary's show, which sees every satellite.
  Costs: a builder in a satellite reads the primary's show to learn what to build.
- B) Yes.
  Why: every repository reports the same.
  Costs: a false report in every satellite for every decision built elsewhere.

### What does check in the primary report for a satellite missing at its path?
depends: primary, satellite, finding, Does check in the primary count a tie in a satellite?=A
unrelated: What does check do when an origin is absent?, Does check fail a planner's commit that reaches outside adrdom?, What does show print when no file is named?, What does check do with a decision row no question names?, What does check do with a test that names a decision that is not current?, What does check do with a fault on a file awaiting approval?, Does check print a line with no move on it?, Does a command name a status of its own?, Does a planning step run without the record of the step before?, What does install write in a satellite?, What does the primary hold of a satellite?, Does check fail a ledger whose rows changed place?, Does the agent build what no current decision covers?, What does a test carry to name its decision?, What does check do with an untested current decision?, Does a command that writes the record run in a satellite?, Does check hold a satellite's brief to the primary's?, Does check in a satellite report an untested decision?, Does install take the path of a satellite?, Does show in a satellite name its primary?, Does uninstall in a satellite drop its row from the primary?, What does check in a satellite read the record from?

Problem: with a satellite absent the count of ties is short, and every decision built there reads as untested.
Standard: a report over a partial input says the input is partial and makes no claim from it.

- A) That the satellite is not found, with its url; no untested decision until it is.
  Why: nothing is there to test; a build move from half a tree would send a builder to build twice.
  Costs: a satellite nobody clones hides the untested decisions of every repository.
- B) That the satellite is not found, with its url; the untested decisions as usual.
  Why: every finding is printed.
  Costs: a build move for every decision built in the absent satellite.
- C) Nothing; the count is what it is.
  Why: check reads what is there.
  Costs: a wrong build move with no word on why.

### Does check hold a satellite's brief to the primary's?
depends: satellite, primary, brief, What does install write in a satellite?=A
unrelated: Does check hold the files install writes?, Does check hold the terms brief to the terms file?, Does a command refuse what check lets pass?, Does check fail a question answered with two different options?, What does check do when an origin is absent?, Does check fail a planner's commit that reaches outside adrdom?, Does check fail two ledger rows on one subject with the same time?, Does check look at the ledger before a push?, Does a finding hold the identifier of the decision it judges?, Does a command name a status of its own?, What does check in the primary report for a satellite missing at its path?, What does the primary hold of a satellite?, Does check at a commit read the staged files?, Does check fail a ledger whose rows changed place?, Does check hold the merge rule install writes for the ledger?, Does check in CI compare the ledger with the commit before the push?, What does check do with a fault on a file awaiting approval?, Does a command that writes the record run in a satellite?, Does check in a satellite report an untested decision?, Does check in the primary count a tie in a satellite?, Does install take the path of a satellite?, Does show in a satellite name its primary?, Does uninstall in a satellite drop its row from the primary?, What does check in a satellite read the record from?

Problem: the copy of the brief in a satellite goes stale when the primary's changes, and nothing says so.
Standard: a copied configuration is checked against its source on every run.

- A) Yes.
  Why: check fails when the copy differs from the primary's file; the move is running install in the satellite.
  Costs: a brief change in the primary stops every satellite's commit until install runs there.
- B) No.
  Why: a copy is written once.
  Costs: a satellite's agent reads a brief the primary left behind.

### Does uninstall in a satellite drop its row from the primary?
depends: satellite, primary, What does install write in a satellite?=*, What does the primary hold of a satellite?=A|B
unrelated: Does uninstall belong to the owner?, Does uninstall remove a file install created?, Does a command compute a move of its own?, Does the reader's file keep a row whose text left the records?, Does install put the embedding model outside the repository?, Does a command name a status of its own?, Does a command stop when the environment gives no session identifier?, Does the tool take the order of ledger rows from their time?, Does install fetch the embedding model?, Does a refusal name the command that clears it?, What does check in the primary report for a satellite missing at its path?, Does an owner command commit?, Does approve stop a terms edit that moves a word an answered question pairs?, Does approve withdraw an asked question struck from the plan?, Does check hold the files install writes?, Does install write a merge rule for the ledger?, Does the owner lift a ban?, Does a command that writes the record run in a satellite?, Does check hold a satellite's brief to the primary's?, Does check in a satellite report an untested decision?, Does check in the primary count a tie in a satellite?, Does install take the path of a satellite?, Does show in a satellite name its primary?, What does check in a satellite read the record from?

Problem: a satellite uninstalled stays on the primary's list, and the primary reports it missing for ever.
Standard: what one command writes on two sides, its reverse removes on both.

- A) Yes.
  Why: install wrote both sides; uninstall leaves neither.
  Costs: the primary's checkout at its path when uninstall runs.
- B) No.
  Why: uninstall touches one repository.
  Costs: the owner edits the primary's list by hand.
