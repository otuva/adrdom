# Blame

How the text of an origin enters the records so that every line of it is accounted for: a blame file under adrdom/blame holds, for one origin, its line ranges and what each range went to. The map is the proof; git keeps the text. Agents write and verify the map, the owner approves it.

### What does a blame file hold?
depends: blame, origin

### Does a blame file cover its whole origin?
depends: blame, origin

### What does a blame row name?
depends: blame, decision, term, topic file

### Does the owner approve a blame file?
depends: blame, approve, owner

### Does check hold a blame file to its origin?
depends: What does a blame file hold?, check, origin

### Does an origin leave the repository while its blame file has an open row?
depends: What does a blame row name?, Does the owner approve a blame file?, origin

### Does the tool stop while a blame file waits for approval?
depends: Does the owner approve a blame file?, blame, check

### Does show print a blame file?
depends: What does a blame row name?, blame
