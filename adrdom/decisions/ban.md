# Ban

Words that must never enter the repository: where the ban list lives, what the scan reads, and
how a ban is lifted.

### Does the repository hold a banned word in plain text?
depends: ef83beae-6857-4200-b251-a0fcc3c72f9d
id: fcc5b6fe-9072-42ea-a25b-2be72e094bac
terms: agents=f90dbd5b-8fbe-4899-8588-f8762795c2e2, banned=ef83beae-6857-4200-b251-a0fcc3c72f9d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d

Problem: a name that must never be public has to be listed somewhere for a scan to find it, and a plain list in a public repository publishes exactly what it guards.
Standard: secret scanners keep plaintext patterns out of the scanned repository or store only digests.

- A) No: the repository holds only the hashes of banned words, so the scan works on every clone.
  Why: a public clone holds nothing to read, and the scan needs no setup per machine.
  Costs: the hash of a short word can be guessed by trying words, so a short name is only lightly hidden.
- B) No: the list lives only on the owner's machine, and the repository holds nothing of it.
  Why: no clone holds even a hash.
  Costs: a fresh machine cannot scan until the owner puts the list there.
- C) Yes: the plaintext list is one file in the repository that agents never read.
  Why: the owner reviews the list by eye.
  Costs: the list is in every public clone.

**Answer:** A

### What does the ban scan read?
depends: ef83beae-6857-4200-b251-a0fcc3c72f9d
id: d8a9dfff-de89-442c-a08c-8276efab3f36
terms: ban=ef83beae-6857-4200-b251-a0fcc3c72f9d, banned=ef83beae-6857-4200-b251-a0fcc3c72f9d

Problem: a banned word that reaches a commit is in history, and removing it means rewriting history; a scan that runs only after a push is late, and one that runs only on one machine misses the others.
Standard: secret scanners run a pre-commit hook for speed and a CI job that does not depend on local setup.

- A) The staged files and the commit message before every commit, and every tracked file in CI on push; a hit stops and names the file and line, never the matched text.
  Why: the word stops where it enters git, and CI catches a machine without the hook.
  Costs: two places run the scan, and CI sees a word only after it is pushed.
- B) The staged files and the commit message before every commit only.
  Why: one place, the earliest one.
  Costs: a machine without the hook lets the word through unseen.
- C) Every tracked file in CI on push only.
  Why: nothing to set up per machine.
  Costs: the word is in history before anything sees it.

**Answer:** A

### Does a term carry its own banned words?
depends: ef83beae-6857-4200-b251-a0fcc3c72f9d, f590059d-b681-40ec-8a1c-c8916d71f6bb
id: c6da863d-5085-402f-815a-9bb832b4df7e
terms: ban=ef83beae-6857-4200-b251-a0fcc3c72f9d, banned=ef83beae-6857-4200-b251-a0fcc3c72f9d, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, term=f590059d-b681-40ec-8a1c-c8916d71f6bb, terms file=528fe6f6-6d77-42d5-9086-017048d1448d

Problem: with banned words kept per term and a separate list for names, two mechanisms scan for the same thing and can disagree.
Standard: one deny list and one scanner.

- A) No: one hashed ban list, each line optionally carrying the identifier of a term, and one scan over it.
  Why: one list, one scanner, one place to lift a ban.
  Costs: the terms file no longer shows which words each term rules out.
- B) Yes: each term lists the words it rules out, beside the hashed list for names.
  Why: the words a term rules out sit next to it.
  Costs: two mechanisms to keep in step.

**Answer:** A

### Does the owner lift a ban?
depends: ef83beae-6857-4200-b251-a0fcc3c72f9d, 1d7e59ea-99e3-4d4b-9928-5dabca62723d
id: ef7ec04d-9cdd-40a1-8246-42a4a0200828
terms: ban=ef83beae-6857-4200-b251-a0fcc3c72f9d, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d

Problem: a ban with no way back leaves a hit the owner intends with no legal move.
Standard: deny lists in secret scanners are edited under the same review as additions, and a removal is logged like an addition.

- A) Yes: unban, an owner command, reads words from standard input, removes their hashes from the list and writes an unbanned event to the ledger.
  Why: the mirror of ban, with the same input and the same kind of event.
  Costs: one more owner command.
- B) Yes, by editing the hash file by hand, with nothing in the ledger.
  Why: nothing new in the tool.
  Costs: a ban lifts with no record of who lifted it.
- C) No: a ban lasts for good.
  Why: nothing to guard.
  Costs: the owner is walled in.

**Answer:** A

### Does a banned word pass when it is only quoted?
depends: d8a9dfff-de89-442c-a08c-8276efab3f36=*, ef83beae-6857-4200-b251-a0fcc3c72f9d
id: 735a32bd-db0c-428e-b65e-2acc750671b0
terms: ban=ef83beae-6857-4200-b251-a0fcc3c72f9d, banned=ef83beae-6857-4200-b251-a0fcc3c72f9d, bans=ef83beae-6857-4200-b251-a0fcc3c72f9d

Problem: a ban with a quotation exception lets the word in wherever someone frames it as a mention, and the scan cannot tell a quotation from a use.
Standard: secret scanners have no mention form; a hit is a hit, and allowing one is an explicit edit.

- A) No: a banned word fails wherever it appears, quoted or not; the sentence is reworded or the word dropped from the quote.
  Why: the scan stays exact, and the word never reaches the repository in any form.
  Costs: a record cannot quote the word it discusses.
- B) Yes: a word inside quotation marks or a code span passes.
  Why: a record can name what it bans.
  Costs: any sentence can carry the word by quoting it.

**Answer:** A

### Does ban refuse a word that appears in the repository?
depends: d8a9dfff-de89-442c-a08c-8276efab3f36, ef83beae-6857-4200-b251-a0fcc3c72f9d
id: 4f04bae4-98e3-48eb-9403-3257268e2d87
