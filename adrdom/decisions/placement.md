# Placement

How a new question is tied to the decisions that exist: the steps of planning a topic file, the neighbours the tool brings, what the reader says about them, the line the agent writes for each, and what happens to a question whose answer set lacks the owner's answer.

### Does the agent write the bodies before approve?
depends: agent, approve, plan
supersedes: Does the owner approve a plan in one step?, Does check fail a plan awaiting approval?

### Does approve hash each question on its own?
depends: approve, question, Does the agent write the bodies before approve?

### Does a depends entry on a question in its file carry an answer set?
depends: answer set, depends, Does the agent write the bodies before approve?

### Does answer remove a question whose answer set lacks the answer?
depends: answer, answer set, Does approve hash each question on its own?
supersedes: Does a depends entry carry the answers it holds under?

### What does answer write for a removed question?
depends: ledger, Does answer remove a question whose answer set lacks the answer?

### What does the tool bring as neighbours for a new heading?
depends: neighbour, decision, definition

### Does git track the embedding index?
depends: embedding index

### What does the agent do to plan a topic file?
depends: agent, plan, What does the tool bring as neighbours for a new heading?

### Does a planning step run without the record of the step before?
depends: check, What does the agent do to plan a topic file?

### Does planning stop without the embedding model?
depends: embedding model, What does the tool bring as neighbours for a new heading?

### Does the tool bring the plan's own questions as neighbours of each other?
depends: plan, What does the reader say about a neighbour of a heading?

### Does every neighbour get a line under its heading?
depends: depends, supersede, What does the agent do to plan a topic file?

### What does the reader say about a neighbour of a heading?
depends: reader, What does the reader say about a neighbour of a question with its body?

### What does the reader say about a neighbour of a question with its body?
depends: reader, What does the agent do to plan a topic file?

### Does the reader's word stop a command?
depends: reader, What does the reader say about a neighbour of a question with its body?

### What does the tool keep of the reader's word?
depends: reader, What does the reader say about a neighbour of a question with its body?

### Does check hold the reader's file as it holds the ledger?
depends: check, What does the tool keep of the reader's word?

### Does planning stop without the reader?
depends: reader, What does the reader say about a neighbour of a heading?

### Does show print the neighbours of a question with the reader's word?
depends: question, What does the tool keep of the reader's word?
