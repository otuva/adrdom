# Reading

The current decisions read against each other: the same reader and the same neighbours as planning, run over the set that exists instead of over a new question.

### Does the tool read every current decision against its neighbours?
depends: decision, neighbour, reader, What does the tool bring as neighbours for a new heading?, What does the tool keep of the reader's word?, Does a decision change after it is answered?
unrelated: Does the tool bring the plan's own questions as neighbours of each other?, Does show print the neighbours of a question with the reader's word?, Does the tool take the order of ledger rows from their time?, Does the agent build what no current decision covers?, Does a supersedes line name every current decision standing on its target?, Does every neighbour get a line under its heading?, Does the ui perform every owner act?, Does the owner override a stop in the tool?, Does the agent that reviews a build name a decision for each behaviour?, Does check read the words of an answered question again?, Does one decision supersede several?, Does check fail a ledger row that was edited?, Does check fail a test that names a decision that is not current?, Does check fail two ledger rows on one subject with the same time?, Does check fail an answered question whose text changed?, What does the reader say about two current decisions?, Does the tool read a pair of decisions in both directions?, Does a commit run the reading of the set?, Does the tool print every pair the reader marks?

Problem: the set holds decisions that repeat or contradict each other and nothing reads them together; a new question meets them, two old ones never meet.
Standard: periodic deduplication of a store with the same comparator used at insert.

- A) Yes.
  Why: the pairs planning never sees, old against old, get the same word and reason as a new question's neighbours.
  Costs: ten reader calls per current decision; rows in the reader's file keyed by text hash and neighbour identifier.
- B) No.
  Why: the set is read only where a new question touches it; no cost beyond planning.
  Costs: known duplicates and contradictions in the set stay until a new question lands beside them.

### What does the reader say about two current decisions?
depends: reader, current, decision, depends, Does the tool read every current decision against its neighbours?=A, What does the reader say about a neighbour of a question with its body?
unrelated: question, supersede, Does every sentence of the brief come from a decision?, Does one decision supersede several?, What does the reader say about a neighbour of a heading?, Does a decision change after it is answered?, Does the word limit hold for the Why line of an option?, answer set, Does check fail a question answered with two different options?, What does a depends line name?, What does a question hold under its heading?, Does the tool read a pair of decisions in both directions?, Does a commit run the reading of the set?, Does the tool print every pair the reader marks?

Problem: both sides are answered, so the whole relation can be read; the words must be ones the agent can act on, and the pair has no new side.
Standard: the same closed set as for a question with its body, applied to two settled texts.

- A) One of same, consistent, inconsistent, depends, unrelated, after one sentence of reason.
  Why: one set for the reader, one file of rows, the same moves: same and inconsistent become a supersede question.
  Costs: depends between two decisions is a finding with no line to write; it is printed and left.
- B) One of same, inconsistent, unrelated, after one sentence of reason.
  Why: only the words that lead to a move; fewer wrong words to read.
  Costs: a missing tie between two old decisions is never named; the set keeps its untied pairs.

### Does the tool read a pair of decisions in both directions?
depends: reader, decision, Does the tool read every current decision against its neighbours?=A, What does the reader say about two current decisions?=A
unrelated: Does one decision supersede several?, Does the tool bring the plan's own questions as neighbours of each other?, Does check fail a question answered with two different options?, Does a decision change after it is answered?, Does the word limit hold for the Why line of an option?, question, Does the tool take the order of ledger rows from their time?, Does the agent build what no current decision covers?, Does the word limit hold for the Costs line of an option?, Does the agent that asks a question build its decision?, What does the reader say about a neighbour of a heading?, Does check fail two ledger rows on one subject with the same time?, Does a depends entry on a question in its file carry an answer set?, Does a supersedes line name every current decision standing on its target?, Does check read the words of an answered question again?, Does a commit run the reading of the set?, Does the tool print every pair the reader marks?

Problem: depends is directional and two decisions have no new side; one reading gives one direction, two readings double the cost.
Standard: read a symmetric relation once; for a directional one, state the direction in the answer.

- A) No.
  Why: each pair is read once in one fixed order; the reader's sentence names which stands on which.
  Costs: the one directional word, depends, is recorded under the pair's first text only.
- B) Yes.
  Why: each decision has its own row for the pair, as in planning; direction is in the row, not the sentence.
  Costs: twice the reader calls, and two words that may disagree for one pair.

### Does a commit run the reading of the set?
depends: agent, Does the tool read every current decision against its neighbours?=A
unrelated: check, Does check at a commit read the staged files?, Does check in CI compare the ledger with the commit before the push?, Does check fail a ledger row that was edited?, Does check hold the reader's file as it holds the ledger?, Does check look at the ledger before a push?, Does check hold the merge rule install writes for the ledger?, Does an owner command commit?, Does check at a merge look for the ledger rows of both branches?, What does the ban scan read?, Does install write outside adrdom?, Does the agent build what no current decision covers?, Does the tool rewrite the ban list from the ledger?, Does install write a merge rule for the ledger?, What does the reader say about two current decisions?, Does the tool read a pair of decisions in both directions?, Does the tool print every pair the reader marks?, Does ban refuse a word that appears in the repository?

Problem: the reading needs the model and the reader and takes minutes; a commit runs on every machine, with neither, in seconds.
Standard: expensive advisory analyses run on demand, not in a commit hook.

- A) No.
  Why: the agent runs it at its own call, after a merge or a round of answers; hooks stay model-free.
  Costs: a set nobody reads stays unread; nothing reminds the agent.
- B) Yes.
  Why: the set is never stale; every commit reads what changed.
  Costs: every commit needs the model and the reader and waits minutes; a hook without them fails.

### Does the tool print every pair the reader marks?
depends: reader, Does the tool read every current decision against its neighbours?=A, What does the reader say about two current decisions?=*, What does the tool keep of the reader's word?
unrelated: Does show print the neighbours of a question with the reader's word?, Does show print every question?, Does check read the words of an answered question again?, Does the tool take the order of ledger rows from their time?, Does the reader's word stop a command?, Does show print a blame file?, Does check hold the reader's file as it holds the ledger?, Does ban refuse a word that appears in the repository?, Does a hook print what check prints on a pass?, Does check fail a ledger row that was edited?, Does a reference line hold its whole list on one line?, Does check fail two ledger rows on one subject with the same time?, What does a question hold under its heading?, Does the tool read a pair of decisions in both directions?, Does a commit run the reading of the set?

Problem: the rows go to a file nobody opens; a pair the reader marks, same, inconsistent or depends, has to reach the agent as a list it can act on.
Standard: a scan prints its findings and the move for each; the store is for the record.

- A) Yes.
  Why: one line per marked pair: both headings, the word, the reason; the move is a supersede question.
  Costs: a long list on a set with many ties; depends pairs are printed with no move.
- B) No.
  Why: the rows are the record; show prints them under each decision.
  Costs: the agent reads show for a hundred decisions to find three findings.
