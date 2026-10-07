# Prints

What a command prints to the one who runs it, in the terminal and on the ui:
the status of what it reads, the findings of the rules it judges, the move
that follows, or the event it wrote; and that status, rule and move are
each computed once.

### Does a command name a status of its own?
depends: status, ledger, topic file, question, term, blame file
unrelated: Does a refusal name the command that clears it?, Does the reader's word stop a command?, What does an owner command look for before it runs?, Does an option under a heading that opens with Does hold one word?, Does a supersedes line name every current decision standing on its target?, Does a term carry its own banned words?, Does an owner command commit?, Does a command write a time later than the last ledger row?, Does approve hash each question on its own?, Does the tool stop while the terms file waits for approval?, Does a reference in a topic file carry the identifier of what it names?, depends, Does show print every question?, Does the agent build what no current decision covers?, What does a blame row name?, Does a command refuse what check passes?, Does a command compute a move of its own?, What does a command that writes nothing print?, What does a command that writes print?, Does check print a line that names no move?, Does show print what a file holds beyond its latest approval?, Does a command print under a flag what it computes from the records?, What does show print with no file named?, What does show print with a file named?, Does the ui show what the terminal prints?, Does a topic file close?

Problem: the statuses of a question are one list in the code, and the statuses of a topic file, a term and a blame file are words each message picks for itself: new or changed, awaiting approval, not approved, removed.
Standard: a state machine with a closed set of statuses, each named once.

- A) No.
  Why: every thing's statuses are one closed list the tool holds, and every command prints a status by that name.
  Costs: a list per thing; a message that needs a status the list lacks waits for a decision.
- B) Yes.
  Why: each command describes what it sees in its own words.
  Costs: two commands name one status two ways, and an agent adds a status no decision gave.

### Does a command refuse what check passes?
depends: check, refusal, approve, ask, Does a refusal name the command that clears it?, Does the owner override a stop in the tool?, Does the reader's word stop a command?, Does ban refuse a word that appears in the repository?, Does ask run on a file whose prose is unapproved?, Does ask refuse an option over a word limit?, What does check do with a fault on a file awaiting approval?, Does approve stop a terms edit that moves a word an answered question pairs?, Does the tool stop while the terms file waits for approval?
unrelated: Does check fail a test that names a decision that is not current?, Does a hook print what check prints on a pass?, Does check hold the ban list equal to the ledger?, Does check hold the files install writes?, Does the agent build what no current decision covers?, Does the owner answer a question with a refusal?, Does a command name a status of its own?, Does a command compute a move of its own?, What does a command that writes nothing print?, What does a command that writes print?, Does check print a line that names no move?, Does show print what a file holds beyond its latest approval?, Does a command print under a flag what it computes from the records?, What does show print with no file named?, What does show print with a file named?, Does the ui show what the terminal prints?, Does check read the words of an answered question again?, Does the tool stop while a blame file waits for approval?

Problem: approve refuses a draft with a refused word that check passes, and refuses a removed file that check and show report as awaiting approval; each command judges the records on its own.
Standard: one judgement per rule on the records, read by every command that acts on it.

- A) No.
  Why: every rule is judged once and read by every judging command; what approve refuses, check reports with the same move.
  Costs: check reads every rule those commands read; a rule on the input of one command, like ban, stays there.
- B) Yes.
  Why: each command judges what it needs when it writes.
  Costs: a draft passes check and the commit, then stops at approve, and the agent learns the rule from the refusal.

### Does a command compute a move of its own?
depends: move, status, Does a command name a status of its own?=*, Does the owner override a stop in the tool?
unrelated: Does the reader's word stop a command?, Does an owner command commit?, Does a command write a time later than the last ledger row?, Does a planning step run without the record of the step before?, Does the tool guard against an agent that works around it on purpose?, Does a commit run the reading of the set?, Does approve hash each question on its own?, Does the tool rewrite the ban list from the ledger?, Does a refusal name the command that clears it?, Does the agent build what no current decision covers?, Does a supersedes line name every current decision standing on its target?, Does check fail two ledger rows on one subject with the same time?, Does the agent write the plan?, What does install write for one agent?, Does a command refuse what check passes?, What does a command that writes nothing print?, What does a command that writes print?, Does check print a line that names no move?, Does show print what a file holds beyond its latest approval?, Does a command print under a flag what it computes from the records?, What does show print with no file named?, What does show print with a file named?, Does the ui show what the terminal prints?

Problem: plan computes its next line on its own and names the owner's approval on a file approved, asked or answered; show computes the move elsewhere, and the two disagree.
Standard: one function per fact, every caller reads it.

- A) No.
  Why: one computation of the move from the status, printed the same by every command that prints one.
  Costs: plan's step lines come from that computation too, and a test holds every command to it.
- B) Yes.
  Why: each command knows its own step best.
  Costs: two commands name two moves for one status, and the agent follows the wrong one.

### What does a command that writes nothing print?
depends: status, move, check, Does a command name a status of its own?=*, Does a command compute a move of its own?=*, What does check do with a fault on a file awaiting approval?, Does the tool print every pair the reader marks?, What does the ui require before an owner act?, Does every ledger row carry the tool version?, What does check do with an untested current decision?
unrelated: Does ban refuse a word that appears in the repository?, Does the reader's word stop a command?, Does the tool rewrite the ban list from the ledger?, Does a command write a time later than the last ledger row?, Does an owner command commit?, Does the tool write the ban list in one fixed order?, What does the ban scan read?, Does check at a commit read the staged files?, Does check read the words of an answered question again?, Does a command refuse what check passes?, What does a command that writes print?, Does check print a line that names no move?, Does show print what a file holds beyond its latest approval?, Does a command print under a flag what it computes from the records?, What does show print with no file named?, What does show print with a file named?, Does the ui show what the terminal prints?, Does a hook print what check prints on a pass?, Does show print every question?

Problem: show, check, plan and read each print lines the one who runs them has no move on, and the move hides among them.
Standard: a read-only command prints what it read and what follows from it, as git status does.

- A) What it reads from the records, its status, the findings, and the move that follows, and nothing else.
  Why: every line is read for a move; the record names each actor's inputs at each step.
  Costs: counts, pass notes and the hint line go; a passing check prints nothing.
- B) Everything it computed on the way: statuses, findings, counts, notes and the move.
  Why: the one who runs it sees that the command ran and what it covered.
  Costs: hundreds of lines around one move; each new command adds lines no decision names.

### What does a command that writes print?
depends: ledger, refusal, move, approve, ask, answer, ban, Does a command refuse what check passes?=*, Does ban refuse a word that appears in the repository?, Does the tool rewrite the ban list from the ledger?, What does install write for one agent?
unrelated: Does a command write a time later than the last ledger row?, Does an owner command commit?, Does every ledger row carry the tool version?, Does check hold the reader's file as it holds the ledger?, Does install write a merge rule for the ledger?, Does the tool take the order of ledger rows from their time?, check, Does a hook print what check prints on a pass?, Does the tool print every pair the reader marks?, Does the reader's word stop a command?, Does show print a blame file?, Does show print every question?, Does show print the neighbours of a question with the reader's word?, What does check do with a fault on a file awaiting approval?, Does the agent build what no current decision covers?, Does approve withdraw an asked question struck from the plan?, Does an answer carry the version of the question the owner read?, Does a command name a status of its own?, Does a command compute a move of its own?, What does a command that writes nothing print?, Does check print a line that names no move?, Does show print what a file holds beyond its latest approval?, Does a command print under a flag what it computes from the records?, What does show print with no file named?, What does show print with a file named?, Does the ui show what the terminal prints?

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
depends: check, move, What does a command that writes nothing print?=*, Does a command compute a move of its own?=*, What does check do with an untested current decision?, What does check do with a fault on a file awaiting approval?
supersedes: Does a hook print what check prints on a pass?, Does a topic file close?, Does the tool stop while a blame file waits for approval?
unrelated: Does check read the words of an answered question again?, Does ban refuse a word that appears in the repository?, Does check fail a ledger whose rows changed place?, Does check fail a test that names a decision that is not current?, Does a reference line hold its list on one line?, Does the reader's word stop a command?, Does check fail a ledger row that was edited?, Does show print every question?, Does check at a commit read the staged files?, Does a command name a status of its own?, Does a command refuse what check passes?, What does a command that writes print?, Does show print what a file holds beyond its latest approval?, Does a command print under a flag what it computes from the records?, What does show print with no file named?, What does show print with a file named?, Does the ui show what the terminal prints?

Problem: on a pass check prints a tier line and a done line per topic file and counts of tested decisions and files awaiting approval, and none carries a move.
Standard: a checker prints its findings, each with the fix, and is silent on a pass.

- A) No.
  Why: every line of check is a finding with its move; a pass prints nothing, by hand or in a hook.
  Costs: hook flag, done, tier and tested lines go; no decision holds that a waiting blame file stops nothing.
- B) Yes.
  Why: the one who runs check sees what it covered.
  Costs: dozens of lines with no move on every run, and twice per commit without a flag.

### Does show print what a file holds beyond its latest approval?
depends: approve, owner, status, topic file, terms file, blame file, Does a command name a status of its own?=*, What does a command that writes nothing print?=*, Does the tool stop while the terms file waits for approval?, Does the owner approve a blame file?
unrelated: Does ask run on a file whose prose is unapproved?, What does check do with a fault on a file awaiting approval?, Does check fail a topic file awaiting approval?, Does the tool stop while a blame file waits for approval?, Does a hook print what check prints on a pass?, Does check hold the reader's file as it holds the ledger?, Does an origin leave the repository while its blame file has an open row?, Does check at a commit read the staged files?, Does an answer carry the version of the question the owner read?, Does the ui perform every owner act?, Does a topic file close?, Does a command refuse what check passes?, Does a command compute a move of its own?, What does a command that writes print?, Does check print a line that names no move?, Does a command print under a flag what it computes from the records?, What does show print with no file named?, What does show print with a file named?, Does the ui show what the terminal prints?, Does show print every question?, Does show print a blame file?

Problem: the owner runs approve without a view of what it approves: show prints a topic file the same before and after approval, never its bodies, and refuses the terms file and a blame path.
Standard: a diff before a commit: the change is read before it is signed.

- A) Yes.
  Why: show takes every file approve takes and prints the headings, lines, bodies, entries or rows beyond its latest approval, marked.
  Costs: show parses the terms file and a blame file; a plan change prints the change, not the file.
- B) No.
  Why: show prints status, and a draft is read with git diff.
  Costs: the owner leaves the tool to see what approve will sign.

### Does a command print under a flag what it computes from the records?
depends: neighbour, reader, move, What does a command that writes nothing print?=*, Does the tool print every pair the reader marks?, Does an answer carry the version of the question the owner read?
supersedes: Does show print the neighbours of a question with the reader's word?
unrelated: Does a hook print what check prints on a pass?, Does show print every question?, Does a command write a time later than the last ledger row?, Does the reader's word stop a command?, Does the tool take the order of ledger rows from their time?, What does check do with a fault on a file awaiting approval?, Does check at a commit read the staged files?, Does every ledger row carry the tool version?, What does the ban scan read?, Does a command name a status of its own?, Does a command refuse what check passes?, Does a command compute a move of its own?, What does a command that writes print?, Does check print a line that names no move?, Does show print what a file holds beyond its latest approval?, What does show print with no file named?, What does show print with a file named?, Does the ui show what the terminal prints?, Does show print a blame file?

Problem: show prints the neighbours of every unasked question with the reader's word in its plain listing, five lines per heading that no move reads.
Standard: a computed view is printed on request, under a flag named for it, as git log --graph and ls -l do.

- A) Yes.
  Why: status and move in the listing; a computed view under a flag, or inside a finding or move needing it.
  Costs: a flag per view; a view with no flag is read from the files the tool writes.
- B) No.
  Why: a command prints the status and the move, and nothing computed, flag or not.
  Costs: the nearest decisions and the reader's word are read from plan's output or the reader's file alone.

### What does show print with no file named?
depends: decision, current, option, move, What does a command that writes nothing print?=*, Does a command print under a flag what it computes from the records?=*
supersedes: Does show print every question?, Does show print a blame file?
unrelated: Does a hook print what check prints on a pass?, Does ban refuse a word that appears in the repository?, What does check do with a fault on a file awaiting approval?, Does git track a file the tool can get again without the owner?, Does the tool print every pair the reader marks?, Does an origin leave the repository while its blame file has an open row?, Does ask run on a file whose prose is unapproved?, Does show print the neighbours of a question with the reader's word?, Does the agent build what no current decision covers?, Does a supersedes line name every current decision standing on its target?, Does an answer carry the version of the question the owner read?, Does a refusal name the command that clears it?, What does install write for one agent?, Does a reference in a topic file carry the identifier of what it names?, Does a command name a status of its own?, Does a command refuse what check passes?, Does a command compute a move of its own?, What does a command that writes print?, Does check print a line that names no move?, Does show print what a file holds beyond its latest approval?, What does show print with a file named?, Does the ui show what the terminal prints?

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
depends: status, question, term, topic file, terms file, blame file, asked, Does the ui perform every owner act?, Does show print what a file holds beyond its latest approval?=*, Does a command print under a flag what it computes from the records?=*, What does a command that writes nothing print?=*, What does a blame row name?, Does a reference in a topic file carry the identifier of what it names?, Does an answer carry the version of the question the owner read?, What does a blame file hold?, Does a command name a status of its own?=*
supersedes: Does show print every question?, Does show print a blame file?
unrelated: Does a hook print what check prints on a pass?, Does the tool print every pair the reader marks?, What does the tool keep of the reader's word?, Does every ledger row carry the tool version?, What does check do with a fault on a file awaiting approval?, Does a topic file close?, Does an origin leave the repository while its blame file has an open row?, Does ban refuse a word that appears in the repository?, Does a command refuse what check passes?, Does a command compute a move of its own?, What does a command that writes print?, Does check print a line that names no move?, What does show print with no file named?, Does the ui show what the terminal prints?, Does show print the neighbours of a question with the reader's word?

Problem: show with a topic file prints each question with its status, its lines and its neighbours, and no body; show refuses the terms file and a blame path, so two of the three drafts the owner approves are read with git.
Standard: a listing command takes every path the writing command takes and prints the record held at that path.

- A) Every question, entry or row with its status, the lines it carries and the body of an asked question.
  Why: one form for every file approve takes; the owner reads the options where the move is to answer.
  Costs: a large blame file prints every row; what stands on a question is a computed view.
- B) Every question, entry or row with its status, the lines it carries and its body.
  Why: the whole file in one run, options and all.
  Costs: a file of twenty answered questions prints hundreds of lines to find one status.

### Does the ui show what the terminal prints?
depends: ui, owner, What does a command that writes nothing print?=*, What does a command that writes print?=*, What does show print with no file named?=*, Does the ui perform every owner act?, Does an answer carry the version of the question the owner read?, Does the adr binary serve the ui?
unrelated: Does show print a blame file?, Does show print every question?, Does the tool print every pair the reader marks?, Does a hook print what check prints on a pass?, Does an option under a heading that opens with Does hold one word?, Does show print the neighbours of a question with the reader's word?, Does the word limit hold for an option's Why line?, What does an owner command look for before it runs?, Does the agent build what no current decision covers?, What does check do with a fault on a file awaiting approval?, What does check do with an untested current decision?, Does a command name a status of its own?, Does a command refuse what check passes?, Does a command compute a move of its own?, Does check print a line that names no move?, Does show print what a file holds beyond its latest approval?, Does a command print under a flag what it computes from the records?, What does show print with a file named?

Problem: the ui copies check's whole output into a block, shows one open question at a time with no count of the rest, and its lines have no decision behind them apart from the owner's acts.
Standard: one view model, two renderings.

- A) Yes.
  Why: the page renders the same statuses, findings, moves and events the commands print, from the one computation.
  Costs: the page shows nothing the terminal cannot; a page form for every line kind.
- B) No.
  Why: the page is laid out for the owner alone, with its own lines.
  Costs: two sets of lines to hold right, and a line on the page with no decision behind it.
