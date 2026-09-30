# Reads

What the agent reads from the tool: what show prints, what the commit hooks
print, what the agent writes on a line the tool turns into identifiers, and
what check tells the agent about a decision whose tier is still open.

### Does show print every question?
depends: question, decision, topic file, Does show print a blame file?

### Does a hook print what check prints on a pass?
depends: check, install, Does check at a commit read the staged files?

### Does approve turn the supersedes line into identifiers?
depends: approve, ask, supersede, Does a reference in a topic file carry the identifier of what it names?, Does one decision supersede several?

### Does check fail an untested decision whose tier is still open?
depends: check, decision, tier, Does check fail a current decision with no test?, Does the agent build on a decision before its tier is fully answered?, Does an owner command commit?
