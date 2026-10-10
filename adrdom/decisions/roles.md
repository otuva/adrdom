# Roles

What the tool knows a session by, what makes a session a planner, and what the tool
holds a planner from once it is one.

### What does the tool know a session by?
depends: session, agent, identifier, Does the tool guard against an agent that works around it on purpose?, What does install write for one agent?
unrelated: Does a command print under a flag what it computes from the records?, Does the tool bring the plan's own questions as neighbours of each other?, Does the tool print every pair the reader marks?, Does the tool read a pair of decisions in both directions?, Does the tool read every current decision against its neighbours?, Does the tool stop while the terms file waits for approval?, What does the tool keep of the reader's word?, status, ui, What makes a session a planner?, What does check do with a test that names a decision that is not current?, Does the ledger hold the planners?, What ends a session's planner role?, What does a comment in code carry to name the decision it follows?, Does a finding name the identifier of the decision it judges?, Does the tool mark a builder?, Does a command stop when the environment gives no session identifier?, Does check fail a planner's commit that reaches outside adrdom?, What does the tool print for a build move in a planner's session?, Does a refusal name the command that clears it?, Does ban refuse a word that appears in the repository?, Does check at a commit read the staged files?, Does install write outside adrdom?, Does the agent build what no current decision covers?, Does the brief name the terms brief?, Does the tool write a terms brief for the agent?, What does the ban scan read?

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

### What makes a session a planner?
depends: planner, session, plan, ask, What does the tool know a session by?=*, Does the agent that asks a question build its decision?, What does the agent do to plan a topic file?
unrelated: Does a planning step run without the record of the step before?, Does a question carry the agent's lean?, Does planning stop without the embedding model?, Does planning stop without the reader?, Does the agent build what no current decision covers?, Does the agent write the plan?, Does the tool bring the plan's own questions as neighbours of each other?, What does check do with a test that names a decision that is not current?, Does the ledger hold the planners?, What ends a session's planner role?, What does a comment in code carry to name the decision it follows?, Does a finding name the identifier of the decision it judges?, Does the tool mark a builder?, Does a command stop when the environment gives no session identifier?, Does check fail a planner's commit that reaches outside adrdom?, What does the tool print for a build move in a planner's session?, Does a refused question stay in its topic file?, Does the agent build on a decision before its tier is fully answered?, Does the tool guard against an agent that works around it on purpose?, What does install write for one agent?, Does the tool write a terms brief for the agent?

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

### What does check do with a test that names a decision that is not current?
depends: check, current, decision, builder, What makes a session a planner?=*, What does a test carry to name its decision?, What does check do with an untested current decision?
supersedes: Does check fail a test that names a decision that is not current?, 23d55c81-2224-487c-a579-298915957a79, 110f471d-ac4b-4cbb-aa7b-3f74ac442d78
unrelated: Does check fail a ledger whose rows changed place?, Does check fail a question answered with two different options?, Does check print a line that names no move?, What does check do with a decision row that no question names?, Does the agent build what no current decision covers?, What does the tool know a session by?, What makes a session a planner?, Does the ledger hold the planners?, What ends a session's planner role?, What does a comment in code carry to name the decision it follows?, Does a finding name the identifier of the decision it judges?, Does the tool mark a builder?, Does a command stop when the environment gives no session identifier?, Does check fail a planner's commit that reaches outside adrdom?, What does the tool print for a build move in a planner's session?, Does a supersedes line name every current decision standing on its target?, Does check fail a ledger row that was edited?, Does check hold the merge rule install writes for the ledger?, Does check in CI compare the ledger with the commit before the push?, What does check do with a fault on a file awaiting approval?, What does check do when an origin is absent?, Does a commit run the reading of the set?

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

### Does the ledger hold the planners?
depends: ledger, planner, What does the tool know a session by?=*, What makes a session a planner?=*, What does a ledger row hold?, Does the tool keep everything it owns under one directory?
unrelated: Does a planning step run without the record of the step before?, Does check fail a ledger row that was edited?, Does check fail an answered question whose text changed?, Does check hold the ban list equal to the ledger?, Does check hold the reader's file as it holds the ledger?, Does check in CI compare the ledger with the commit before the push?, Does planning stop without the reader?, Does the tool rewrite the ban list from the ledger?, Does the tool take the order of ledger rows from their time?, What does check do with a test that names a decision that is not current?, What ends a session's planner role?, What does a comment in code carry to name the decision it follows?, Does a finding name the identifier of the decision it judges?, Does the tool mark a builder?, Does a command stop when the environment gives no session identifier?, Does check fail a planner's commit that reaches outside adrdom?, What does the tool print for a build move in a planner's session?, Does an owner command commit?, Does check hold the merge rule install writes for the ledger?, Does every ledger row carry the tool version?, Does install write a merge rule for the ledger?

Problem: the role has to outlive the command that assigns it, and the ledger is the one record the tool appends to.
Standard: per-machine runtime state stays out of version control.

- A) No.
  Why: the role lives in an untracked file under adrdom/state, holding the identifier and the word.
  Costs: a second store beside the ledger; a fresh clone or a new worktree knows no planner.
- B) Yes.
  Why: one record; a planner's role reaches every clone.
  Costs: a new ledger event, a row per session, and a role that means nothing off its machine.


### What ends a session's planner role?
depends: planner, session, What does the tool know a session by?=A, What makes a session a planner?=*, Does the agent that asks a question build its decision?
unrelated: Does a planning step run without the record of the step before?, Does approve withdraw an asked question struck from the plan?, Does ask stop a question outside the plan?, Does planning stop without the embedding model?, Does planning stop without the reader?, Does the agent build what no current decision covers?, What does the agent do to plan a topic file?, What may leave a topic file by a plan change?, withdraw, What does the tool know a session by?, What does check do with a test that names a decision that is not current?, Does the ledger hold the planners?, What does a comment in code carry to name the decision it follows?, Does a finding name the identifier of the decision it judges?, Does the tool mark a builder?, Does a command stop when the environment gives no session identifier?, Does check fail a planner's commit that reaches outside adrdom?, What does the tool print for a build move in a planner's session?, Does a question carry the agent's lean?, Does the agent build on a decision before its tier is fully answered?, Does the agent write the plan?, Does the tool guard against an agent that works around it on purpose?, Does a refused question stay in its topic file?, status

Problem: a planner that may build once its questions are answered is the asking agent building its decision.
Standard: a role bound to a process ends with the process.

- A) The end of the session; the role stays on file and a new session brings a new identifier.
  Why: one session, one role; the planner goes on to other topics as a planner.
  Costs: a planner that would build another topic opens a new session.
- B) The approval of the plan the session wrote: it then builds topics it did not ask.
  Why: a session plans once and builds elsewhere after.
  Costs: the hook must tell one topic's build from another's, and a commit names no decision.

### What does a comment in code carry to name the decision it follows?
depends: decision, identifier, check, agent, What does a test carry to name its decision?, What does check do with an untested current decision?, What does check do with a test that names a decision that is not current?=*
unrelated: Does a command name a status of its own?, Does a depends entry on a question in its file carry an answer set?, Does a finding carry the identifier of the decision it judges?, Does a reference in a topic file carry the identifier of what it names?, Does a supersedes line name every current decision standing on its target?, Does check fail a test that names a decision that is not current?, Does check read the words of an answered question again?, Does the adr binary serve the ui?, Does the agent build what no current decision covers?, Does the word limit hold for an option's Why line?, What does check do with a fault on a file awaiting approval?, What does check do with a decision row that no question names?, What does a comment in code carry to name a decision?, depends, supersede, What does the tool know a session by?, What makes a session a planner?, Does the ledger hold the planners?, What ends a session's planner role?, Does a finding name the identifier of the decision it judges?, Does the tool mark a builder?, Does a command stop when the environment gives no session identifier?, Does check fail a planner's commit that reaches outside adrdom?, What does the tool print for a build move in a planner's session?

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

### Does a finding name the identifier of the decision it judges?
depends: finding, decision, identifier, current, supersede, What does a test carry to name its decision?, What does a finding name?, What does check do with a test that names a decision that is not current?=*
unrelated: Does a depends entry on a question in its file carry an answer set?, Does a question keep its identifier when its heading is reworded?, Does a reference in a topic file carry the identifier of what it names?, Does a supersedes line name every current decision standing on its target?, Does a term carry its own banned words?, Does check fail a test that names a decision that is not current?, Does check read the words of an answered question again?, Does every sentence of the brief come from a decision?, Does the agent build what no current decision covers?, Does the cap count a decision that an open question supersedes?, Does the tool read a pair of decisions in both directions?, What does a term hold?, What does check do with an untested current decision?, Does a finding carry the identifier of the decision it judges?, What does check do with a decision row that no question names?, Does the agent that reviews a build name a decision for each behaviour?, What does a comment in code carry to name a decision?, What does the tool know a session by?, What makes a session a planner?, Does the ledger hold the planners?, What ends a session's planner role?, What does a comment in code carry to name the decision it follows?, Does the tool mark a builder?, Does a command stop when the environment gives no session identifier?, Does check fail a planner's commit that reaches outside adrdom?, What does the tool print for a build move in a planner's session?, What does a blame file hold of its origin?

Problem: a finding is tagged with a name the code picks, tells, plan, ids; nothing ties a message to the decision it judges, and a message with no decision behind it is written like the rest.
Standard: every diagnostic carries a stable code that documentation and tests key on.

- A) Yes.
  Why: a finding with no current decision behind it cannot be built; one test holds every finding to its decision.
  Costs: the identifier stays in the code and the test, never on the line; a superseded decision re-tags its findings.
- B) No.
  Why: the code names its rules as it likes; the record and the messages stay apart.
  Costs: a message with no decision behind it passes every gate and is caught at review alone.

### Does the tool mark a builder?
depends: builder, planner, approve, answer, What makes a session a planner?=*, What ends a session's planner role?=*, Does the agent build what no current decision covers?, ask
unrelated: Does every ledger row carry the tool version?, Does git track a file the tool can get again without the owner?, Does the agent that reviews a build name a decision for each behaviour?, Does the owner override a stop in the tool?, Does the tool bring the plan's own questions as neighbours of each other?, Does the tool guard against an agent that works around it on purpose?, Does the tool print every pair the reader marks?, Does the tool stop while the terms file waits for approval?, ui, What does the tool know a session by?, What does check do with a test that names a decision that is not current?, Does the ledger hold the planners?, What does a comment in code carry to name the decision it follows?, Does a finding name the identifier of the decision it judges?, Does a command stop when the environment gives no session identifier?, Does check fail a planner's commit that reaches outside adrdom?, What does the tool print for a build move in a planner's session?, Does an owner command commit?, Does check hold the merge rule install writes for the ledger?, Does install write a merge rule for the ledger?, Does install write outside adrdom?, Does the agent build on a decision before its tier is fully answered?, Does the agent write the plan?, Does the owner approve a blame file?, What does check do with a fault on a file awaiting approval?, Does the owner answer a question with a refusal?

Problem: a session that commits code and then writes the questions on what it committed asks for its own build.
Standard: a gate is placed where no other gate stands.

- A) No.
  Why: a question from any session passes approve and answer before it binds; the owner reads it there.
  Costs: a builder may write the questions on what it built, and the owner catches it reading.
- B) Yes.
  Why: the first commit outside adrdom marks the session, and plan and ask refuse it after.
  Costs: a second mark, a hook that writes, and a read in plan and ask, before gates the owner holds anyway.


### Does a command stop when the environment gives no session identifier?
depends: session, identifier, owner, What does the tool know a session by?=A, Does the ledger hold the planners?=*, What does an owner command look for before it runs?, Does the tool guard against an agent that works around it on purpose?, Does a refusal name the command that clears it?, Does an owner command commit?
unrelated: Does a command compute a move of its own?, Does a command name a status of its own?, Does a command print under a flag what it computes from the records?, Does a command refuse what check passes?, Does check print a line that names no move?, Does the reader's word stop a command?, What does a command that writes nothing print?, What does a command that writes print?, What makes a session a planner?, What does check do with a test that names a decision that is not current?, What ends a session's planner role?, What does a comment in code carry to name the decision it follows?, Does a finding name the identifier of the decision it judges?, Does the tool mark a builder?, Does check fail a planner's commit that reaches outside adrdom?, What does the tool print for a build move in a planner's session?, Does install write outside adrdom?, Does the agent build what no current decision covers?, Does the tool rewrite the ban list from the ledger?, What does install write for one agent?, Does the owner override a stop in the tool?

Problem: the owner's terminal and an agent program the tool does not know give no identifier.
Standard: an optional credential is absent, not invalid; absence takes the open path.

- A) No.
  Why: a session the tool cannot name is held by nothing, the owner's own commits among them.
  Costs: an agent program that gives no identifier plans and builds in one session unseen.
- B) Yes.
  Why: every session that plans is known; none plans unseen.
  Costs: the owner, every test and the push check run no command, and the hook stops the owner's commits.


### Does check fail a planner's commit that reaches outside adrdom?
depends: check, planner, What does the tool know a session by?=A, What makes a session a planner?=*, What ends a session's planner role?=*, Does the ledger hold the planners?=*, Does a command stop when the environment gives no session identifier?=*, Does the agent that asks a question build its decision?, Does the tool guard against an agent that works around it on purpose?, Does install write outside adrdom?
unrelated: Does a commit run the reading of the set?, Does check at a merge look for the ledger rows of both branches?, Does check fail a ledger whose rows changed place?, Does check fail a test that names a decision that is not current?, Does check fail a topic file awaiting approval?, Does check hold the files install writes?, Does check hold the merge rule install writes for the ledger?, Does check in CI compare the ledger with the commit before the push?, Does check look at the ledger before a push?, What does check do with a fault on a file awaiting approval?, What does the tool know a session by?, What does check do with a test that names a decision that is not current?, What does a comment in code carry to name the decision it follows?, Does a finding name the identifier of the decision it judges?, Does the tool mark a builder?, Does a command stop when the environment gives no session identifier?, What does the tool print for a build move in a planner's session?, Does install write a merge rule for the ledger?, Does the agent build what no current decision covers?

Problem: the record says the asking agent does not build, and nothing holds the commit that breaks it.
Standard: a policy on changes is enforced at the commit hook, where a change is visible as paths.

- A) Yes.
  Why: everything a planner writes lives under adrdom; a path outside it in a planner's commit is a build.
  Costs: a planner that commits a brief edit, a tie or an install is refused; a new session does that.
- B) No.
  Why: the brief alone holds the rule, as it holds every rule on the agent's conduct.
  Costs: the rule stays unenforced; the asking agent builds its decision with nothing in the way.

### What does the tool print for a build move in a planner's session?
depends: move, planner, builder, What makes a session a planner?=*, Does check fail a planner's commit that reaches outside adrdom?=A, What does show print with no file named?, What does show print of a file under adrdom?, Does a command compute a move of its own?
unrelated: Does a command print under a flag what it computes from the records?, Does a planning step run without the record of the step before?, Does check print a line that names no move?, Does show print what a file holds beyond its latest approval?, Does the agent build what no current decision covers?, Does the tool print every pair the reader marks?, Does the tool take the order of ledger rows from their time?, Does the ui show what the terminal prints?, What does check do with a fault on a file awaiting approval?, What does the tool know a session by?, What does check do with a test that names a decision that is not current?, Does the ledger hold the planners?, What ends a session's planner role?, What does a comment in code carry to name the decision it follows?, Does a finding name the identifier of the decision it judges?, Does the tool mark a builder?, Does a command stop when the environment gives no session identifier?, Does a commit run the reading of the set?, Does a refused question stay in its topic file?, Does check at a commit read the staged files?, Does the agent write the plan?, Does the tool rewrite the ban list from the ledger?, What does the ban scan read?, Does the tool bring the plan's own questions as neighbours of each other?, status, Does the owner override a stop in the tool?, Does the tool guard against an agent that works around it on purpose?

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
