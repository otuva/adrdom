# Install

What install puts into a repository, what check holds it to, and how the owner takes it out.

### Does the tool keep everything it owns under one directory?
depends: 1b98cdea-dc1b-4789-8f67-68d130d7a49d
id: 16a71892-b348-4b9b-82c5-a9d375187bd3
terms: ban=ef83beae-6857-4200-b251-a0fcc3c72f9d, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, topic files=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: a tool that scatters its files across a repository leaves no single place to read what it governs, and its files mix with the repository's own.
Standard: tools that store state in a repository put it under one named directory, such as .github or .husky, or under a configured path.

- A) Yes: one visible adrdom directory at the root, with topic files under adrdom/decisions and the ledger and ban list under adrdom/state.
  Why: one place to read, with the state beside the records it holds.
  Costs: the directory name is fixed in every repository.
- B) No: topic files under docs/decisions and state in a hidden .adr directory.
  Why: follows common documentation layouts and hides machine state.
  Costs: two places to look, and hidden state is easy to miss in review.
- C) A path the owner sets in a config file.
  Why: fits any layout.
  Costs: a config file to read before anything else, and every repository differs.

**Answer:** A

### Does install write outside adrdom?
depends: 1b98cdea-dc1b-4789-8f67-68d130d7a49d
id: 4f925b13-ad16-48b8-acd1-3e30d6e308b6
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, agents=f90dbd5b-8fbe-4899-8588-f8762795c2e2, install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d

Problem: the tool needs git hooks to run and a brief every agent reads; both live outside adrdom, and a tool that writes files it does not own surprises the repository.
Standard: hook managers such as husky keep hooks in a tracked directory and point git at it; agent instructions live in a root AGENTS.md that most agents read.

- A) Yes, only its git hooks and the brief: hooks in a tracked .githooks directory with git's hooksPath pointed at it, and the brief as a marked section of AGENTS.md; anything for one agent needs a flag naming that agent.
  Why: the two things every repository needs, and nothing agent-specific without a flag.
  Costs: AGENTS.md and the hook directory are shared with anything else the repository runs.
- B) No: install writes only adrdom and prints the rest for the owner to add.
  Why: the tool touches nothing it does not own.
  Costs: a missed step leaves the hooks off or agents without the brief, silently.
- C) Yes, everything for every known agent at once.
  Why: one command and no flags.
  Costs: files for agents the repository never uses.

**Answer:** A

### Does check hold the files install writes?
depends: 1b98cdea-dc1b-4789-8f67-68d130d7a49d, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de
id: 6ef9eca6-fd8b-4211-aae7-00b37a2a9245
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, agents=f90dbd5b-8fbe-4899-8588-f8762795c2e2, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, installed=1b98cdea-dc1b-4789-8f67-68d130d7a49d

Problem: an agent can edit the brief or remove a deny entry, and the repository then tells agents something the tool never wrote.
Standard: generated files are compared with their generator in CI, and drift fails.

- A) Yes: check fails when the AGENTS.md brief section or an agent's installed block differs from what install writes; the move is running install again.
  Why: the brief every agent reads stays the tool's text.
  Costs: a local change to the brief fails every commit.
- B) No: install writes them once and nothing looks again.
  Why: the repository may adapt them.
  Costs: the brief drifts and nobody notices.

**Answer:** A

### Does uninstall belong to the owner?
depends: 1d7e59ea-99e3-4d4b-9928-5dabca62723d, 1b98cdea-dc1b-4789-8f67-68d130d7a49d
id: 292359a4-0512-465b-b6a7-79838e66e5a5
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d

Problem: a command that removes the tool's hooks, brief and agent blocks takes every guard out of the repository in one run, and an agent may run it as routine cleanup.
Standard: removing a policy control is an administrator action, separate from the users the control governs.

- A) Yes: uninstall is an owner command; it removes what install wrote outside adrdom, each piece only while it still matches what install wrote, and leaves adrdom in place.
  Why: the agent cannot take the governance out of the repository it works in, and nothing the repository changed by hand is deleted.
  Costs: removing the tool needs the owner at a terminal.
- B) No: any agent may run uninstall.
  Why: cleanup is a routine task.
  Costs: one command removes every guard.

**Answer:** A

### What does install write for one agent?
depends: 4f925b13-ad16-48b8-acd1-3e30d6e308b6=A|C, f90dbd5b-8fbe-4899-8588-f8762795c2e2
id: ae63df95-0643-4108-af65-1e7a0009d631
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, agents=f90dbd5b-8fbe-4899-8588-f8762795c2e2, install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d

Problem: each agent reads instructions and permission rules from its own files; without a block per agent, some agents never load the brief and nothing in their own rules forbids an owner command.
Standard: tools that support several agents write each agent's own configuration in that agent's own format.

- A) Each agent's own files, derived from the one list of owner commands: with the flag naming an agent, that agent's instruction file imports AGENTS.md, created when missing, and that agent's permission file forbids every owner command.
  Why: one list feeds every agent's block, so a new owner command reaches all of them.
  Costs: each new agent needs a flag and its own format in the tool.
- B) The brief only; each agent's own files are the repository's business.
  Why: the tool stays neutral between agents.
  Costs: an agent that does not read AGENTS.md never sees the brief, and no agent's own rules forbid owner commands.

**Answer:** A

### Does uninstall remove a file install created?
depends: 292359a4-0512-465b-b6a7-79838e66e5a5=*, 1b98cdea-dc1b-4789-8f67-68d130d7a49d
id: c304b7da-1843-43ad-930c-b01b3310483d
terms: install=1b98cdea-dc1b-4789-8f67-68d130d7a49d

Problem: install may create a file only to hold its own lines; removing those lines leaves an empty file the repository never had.
Standard: package uninstallers remove the files they created and leave the files the user changed.

- A) Yes: when a file holds nothing but what install wrote, uninstall deletes it, and a directory install created goes too once empty; a file with anything else in it keeps the rest.
  Why: after uninstall the repository looks as it did before install.
  Costs: uninstall tells a file it created from one it only edited by the content alone.
- B) No: uninstall removes its lines and leaves the file, even empty.
  Why: uninstall never deletes a file.
  Costs: empty files the repository never had.

**Answer:** A

### Does install fetch the embedding model?
depends: a7b0b910-57f6-4c59-b904-b42f78db63f7, 1b98cdea-dc1b-4789-8f67-68d130d7a49d, 4f925b13-ad16-48b8-acd1-3e30d6e308b6
unrelated: 21cdad59-4f0c-42a5-b4f3-6af946637df3, 19e1c023-a678-4842-a414-bb0e45c41601, 99619e27-19b6-47ef-83b9-c79ac30ee2f0, db56a285-946f-48b1-b073-bb1e90192185, bf7da2dd-d8a7-41b8-9898-11cf68d0e833, fa0b0e24-b7cf-471b-94b6-6d167e8ad269, daec9ffd-676f-4111-a359-2827bfd4527c, 0d977e39-a657-4596-8eca-8ceb335f3081, 34e81f57-9b3c-4dc4-a2ba-ca227523177d, 6ef9eca6-fd8b-4211-aae7-00b37a2a9245, 76c6adc6-c018-4df4-b49a-ddcd0a03a48c, 459c1b77-82dd-469f-a60c-784b6f3d7b81, 5418b256-dfae-439a-96c1-57d82380cd1e
id: e1c305af-af94-494a-9fdb-3faffafa0e06
terms: embedding model=a7b0b910-57f6-4c59-b904-b42f78db63f7, install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf

Problem: the embedding model is a large file outside git; a plan that fetches it mid-command stops halfway without the network, and nothing names the step that gets it.
Standard: a setup step fetches what a command needs; a command does not install its own dependencies.

- A) Yes.
  Why: one named step, run once; a plan either has the model or stops and names the step.
  Costs: one more install flag, and a plan on a machine without it waits for install.
- B) No.
  Why: no extra step; the first plan fetches it.
  Costs: a plan starts with a long download, and stops halfway without the network.

**Answer:** A

### Does install put the embedding model outside the repository?
depends: a7b0b910-57f6-4c59-b904-b42f78db63f7, 1b98cdea-dc1b-4789-8f67-68d130d7a49d, 4f925b13-ad16-48b8-acd1-3e30d6e308b6, e1c305af-af94-494a-9fdb-3faffafa0e06=A
unrelated: 21cdad59-4f0c-42a5-b4f3-6af946637df3, db07a54d-4d80-4035-a9ef-eabcc0c872b3, 76c6adc6-c018-4df4-b49a-ddcd0a03a48c, 99619e27-19b6-47ef-83b9-c79ac30ee2f0, db56a285-946f-48b1-b073-bb1e90192185, fcc5b6fe-9072-42ea-a25b-2be72e094bac, daec9ffd-676f-4111-a359-2827bfd4527c, 16a71892-b348-4b9b-82c5-a9d375187bd3, 292359a4-0512-465b-b6a7-79838e66e5a5, ae63df95-0643-4108-af65-1e7a0009d631, 149c9a73-16eb-46aa-9806-2950423ee1a2, 0d977e39-a657-4596-8eca-8ceb335f3081, 5418b256-dfae-439a-96c1-57d82380cd1e
id: 459c1b77-82dd-469f-a60c-784b6f3d7b81
terms: embedding model=a7b0b910-57f6-4c59-b904-b42f78db63f7, install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, plans=94677ed8-eaf6-46e2-bc71-7392fa0805bf

Problem: a model fetched into each repository's cache is one copy per repository, hundreds of megabytes each, fetched again for every clone.
Standard: a downloaded artefact shared by projects lives in one user cache directory, such as ~/.cache or ~/.cargo.

- A) Yes.
  Why: one copy per machine serves every repository; a new clone plans at once.
  Costs: a file of the tool lives where no repository shows it, and no one repository's uninstall removes it.
- B) No.
  Why: everything the tool uses for a repository sits beside it, under adrdom/cache.
  Costs: one copy per repository, fetched again for each clone.

**Answer:** A

### What does the reader run?
depends: be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, ae63df95-0643-4108-af65-1e7a0009d631
unrelated: bd94a89f-7142-4dab-b757-6482a718a0a0, e78b8297-fd89-4e09-9c3e-9eeb0fd0f654, 60341be0-fd05-4281-92d7-668584a8539a, a8034401-420c-45d7-8f43-60c1b9f3baf9, 27bd0855-a6c8-4cda-a82b-bf95022a14b0, a9e9e370-7768-4a79-b67b-7fe0926b5670, 1af76f91-5644-42ef-97ea-8f70f934c26c, 30ca5d13-e68b-465f-aa84-ea4000c9330a, 55a6a2d8-03af-4db9-a4c7-e2790e38357e, f90dbd5b-8fbe-4899-8588-f8762795c2e2, e53b805c-9272-4b89-985d-61426fb5fa48, 98d5d389-8315-41da-9970-666e31f3a94d, c3127af2-4c96-48d5-a164-bac49a9a09ec, 9a2ff21b-c20b-4c63-95f2-a7a3cbe236b8, 4f925b13-ad16-48b8-acd1-3e30d6e308b6, 6ef9eca6-fd8b-4211-aae7-00b37a2a9245, e1c305af-af94-494a-9fdb-3faffafa0e06, 459c1b77-82dd-469f-a60c-784b6f3d7b81
id: 5418b256-dfae-439a-96c1-57d82380cd1e
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, agents=f90dbd5b-8fbe-4899-8588-f8762795c2e2, install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, installed=1b98cdea-dc1b-4789-8f67-68d130d7a49d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, reader=be0d6f5c-ae1b-4753-8ec7-0f4b03d84641

Problem: the reader is a program on the machine; a reader fixed to one program stops planning on a machine that runs another agent.
Standard: a tool that integrates with several agents calls the one configured for the repository.

- A) The program of the agent named at install, found by the files install wrote for it.
  Why: the agent flag at install names the reader too; a new agent is one flag and one program.
  Costs: a repository installed for no agent has no reader; two agents installed means the tool picks one.
- B) One fixed program the tool names.
  Why: one reader everywhere, one prompt tuned once.
  Costs: a machine without that program cannot plan, whatever agent it runs.
- C) A program named in a setting the owner writes.
  Why: any program the owner picks.
  Costs: one more file to read, and a setting the agent can edit.

**Answer:** A

### Does a repository keep hooks of its own beside the tool's?
depends: 1b98cdea-dc1b-4789-8f67-68d130d7a49d, 4f925b13-ad16-48b8-acd1-3e30d6e308b6, 6ef9eca6-fd8b-4211-aae7-00b37a2a9245
unrelated: 0d977e39-a657-4596-8eca-8ceb335f3081, 459c1b77-82dd-469f-a60c-784b6f3d7b81, 149c9a73-16eb-46aa-9806-2950423ee1a2, 19e1c023-a678-4842-a414-bb0e45c41601, 99619e27-19b6-47ef-83b9-c79ac30ee2f0, e1c305af-af94-494a-9fdb-3faffafa0e06, dd50488e-a8cd-4778-aecc-72295ffd8671, 16a71892-b348-4b9b-82c5-a9d375187bd3, dfd32ef6-c2ac-40ef-b8b3-9c58732516b1, 34e81f57-9b3c-4dc4-a2ba-ca227523177d, 76c6adc6-c018-4df4-b49a-ddcd0a03a48c, db56a285-946f-48b1-b073-bb1e90192185, 6ac1dde1-fe11-47af-861a-334dbf917b11, 60cfa0ba-9085-43b8-9ed4-fd94808c33e0, 897b52a7-f6d2-467e-ba14-9f4ce4865664, db76c9a0-b4fc-4aa2-8afb-47cd78ef7138
id: d6112bd7-1649-4a47-ab3f-0681e4d0e8f6
terms: check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d

Problem: a repository with a hook of its own loses it when install points git's hooksPath at the tool's directory.
Standard: hook managers such as pre-commit and lefthook keep the hook they find and run it after their own.

- A) Yes.
  Why: the repository's lint or test hook keeps running, and check runs in front of it.
  Costs: a second hook file per hook name in the tool's directory, and a hook text that calls it.
- B) No.
  Why: one hook per name, the tool's, and check judges the whole directory.
  Costs: install silences a hook the repository had, and the owner finds it another home by hand.

**Answer:** A

### Does the hook run the repository's own hook once check passes?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, 1b98cdea-dc1b-4789-8f67-68d130d7a49d, d6112bd7-1649-4a47-ab3f-0681e4d0e8f6=A
unrelated: 19e1c023-a678-4842-a414-bb0e45c41601, 34e81f57-9b3c-4dc4-a2ba-ca227523177d, db07a54d-4d80-4035-a9ef-eabcc0c872b3, 8cba4463-6ee3-4590-a819-94462ef80224, a26876ba-5789-4d05-965e-853d00d9edd2, 80b89c24-7d65-43f4-bb03-55e9d85c327d, 76c6adc6-c018-4df4-b49a-ddcd0a03a48c, 0d977e39-a657-4596-8eca-8ceb335f3081, dfd32ef6-c2ac-40ef-b8b3-9c58732516b1, cd83e277-ebf0-4458-a77a-c526ab4cfeca, 4f925b13-ad16-48b8-acd1-3e30d6e308b6, 05141f49-d9d2-407c-b60c-ebb195a05864, d8a9dfff-de89-442c-a08c-8276efab3f36, 60cfa0ba-9085-43b8-9ed4-fd94808c33e0, 897b52a7-f6d2-467e-ba14-9f4ce4865664, db76c9a0-b4fc-4aa2-8afb-47cd78ef7138, 4f04bae4-98e3-48eb-9403-3257268e2d87
id: 6ac1dde1-fe11-47af-861a-334dbf917b11
terms: check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, refuses=0942e445-0163-423e-8ba7-84dd8ac07045

Problem: the hook text is fixed and check judges it line by line, so a line the repository adds to it fails every commit.
Standard: a chaining hook runs the next hook with its own arguments and input and returns its exit code.

- A) Yes.
  Why: check refuses first; the repository's hook sees only what check passed, with git's arguments and input.
  Costs: the hook text saves git's input for both, since the push check reads it; the own hook fails last.
- B) No.
  Why: the repository's hook runs first and may format or stage before check reads the staged files.
  Costs: check judges what the own hook left, and a hook that stages files changes the commit under check.

**Answer:** A

### Does install move a hook it finds into the repository's own hook file?
depends: 1b98cdea-dc1b-4789-8f67-68d130d7a49d, 6ac1dde1-fe11-47af-861a-334dbf917b11=A, d6112bd7-1649-4a47-ab3f-0681e4d0e8f6=A
unrelated: 459c1b77-82dd-469f-a60c-784b6f3d7b81, 19e1c023-a678-4842-a414-bb0e45c41601, e1c305af-af94-494a-9fdb-3faffafa0e06, 937bc0a6-691d-4d67-91c6-cc9e956ebc96, 0d977e39-a657-4596-8eca-8ceb335f3081, 76c6adc6-c018-4df4-b49a-ddcd0a03a48c, aac5c313-dedd-466b-a0c3-ead5e99ce913, db56a285-946f-48b1-b073-bb1e90192185, 4f925b13-ad16-48b8-acd1-3e30d6e308b6, dfd32ef6-c2ac-40ef-b8b3-9c58732516b1, 34e81f57-9b3c-4dc4-a2ba-ca227523177d, 6ef9eca6-fd8b-4211-aae7-00b37a2a9245, c304b7da-1843-43ad-930c-b01b3310483d, 897b52a7-f6d2-467e-ba14-9f4ce4865664, db76c9a0-b4fc-4aa2-8afb-47cd78ef7138
id: 60cfa0ba-9085-43b8-9ed4-fd94808c33e0
terms: install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, move=d904dc0d-231f-4e93-ac74-7babe888107d, moves=d904dc0d-231f-4e93-ac74-7babe888107d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d

Problem: a hook under .git/hooks stops running once hooksPath points elsewhere, and a hook in the tool's directory is overwritten.
Standard: pre-commit renames the hook it finds to .legacy and runs it from its own hook.

- A) Yes.
  Why: the hook runs from the first commit after install, with nothing to type by hand.
  Costs: install writes a tracked file it does not own, and a hook that relies on its old path breaks.
- B) No.
  Why: install writes only its own files, and the owner moves the hook knowing what moved.
  Costs: the hook install found is silent until the owner moves it by hand.

**Answer:** A

### Does install stop when git's hooksPath points away from its hook directory?
depends: 1b98cdea-dc1b-4789-8f67-68d130d7a49d, d6112bd7-1649-4a47-ab3f-0681e4d0e8f6=A, 6ac1dde1-fe11-47af-861a-334dbf917b11=A
unrelated: 19e1c023-a678-4842-a414-bb0e45c41601, 459c1b77-82dd-469f-a60c-784b6f3d7b81, 0d977e39-a657-4596-8eca-8ceb335f3081, 937bc0a6-691d-4d67-91c6-cc9e956ebc96, 76c6adc6-c018-4df4-b49a-ddcd0a03a48c, db56a285-946f-48b1-b073-bb1e90192185, db07a54d-4d80-4035-a9ef-eabcc0c872b3, 6ef9eca6-fd8b-4211-aae7-00b37a2a9245, 4f925b13-ad16-48b8-acd1-3e30d6e308b6, c304b7da-1843-43ad-930c-b01b3310483d, a8280dd1-4bfa-4eeb-9656-1cd06381590a, 60cfa0ba-9085-43b8-9ed4-fd94808c33e0, db76c9a0-b4fc-4aa2-8afb-47cd78ef7138
id: 897b52a7-f6d2-467e-ba14-9f4ce4865664
terms: install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, refuses=0942e445-0163-423e-8ba7-84dd8ac07045

Problem: a hooksPath another hook manager set is repointed at the tool's directory without a word, and that manager sets it back at its next run.
Standard: pre-commit refuses to install while core.hooksPath is set.

- A) Yes.
  Why: two managers of one git setting cannot both hold it; the owner picks one and knows why.
  Costs: install stops in a husky repository until the owner removes the other manager or routes it through the own hook.
- B) No.
  Why: install finishes, and the own hook file calls the hook at the old path, so both run.
  Costs: the other manager sets hooksPath back at its next run and the tool's hooks go silent.

**Answer:** A

### Does check hold git's hooksPath?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, d6112bd7-1649-4a47-ab3f-0681e4d0e8f6=A, 6ef9eca6-fd8b-4211-aae7-00b37a2a9245
unrelated: 19e1c023-a678-4842-a414-bb0e45c41601, 76c6adc6-c018-4df4-b49a-ddcd0a03a48c, 34e81f57-9b3c-4dc4-a2ba-ca227523177d, 0d977e39-a657-4596-8eca-8ceb335f3081, db07a54d-4d80-4035-a9ef-eabcc0c872b3, a26876ba-5789-4d05-965e-853d00d9edd2, bd83aa22-94fc-49ac-b215-a82990ff8c65, 1c27f2d6-ab62-4c5a-91b2-1488722f3c48, 4f925b13-ad16-48b8-acd1-3e30d6e308b6, c10c3a49-e993-4c9f-a46b-4a1ba3646dcf, 0c3d498a-c540-47f3-8384-050fc044c109, d8a9dfff-de89-442c-a08c-8276efab3f36, 6ac1dde1-fe11-47af-861a-334dbf917b11, 60cfa0ba-9085-43b8-9ed4-fd94808c33e0, 897b52a7-f6d2-467e-ba14-9f4ce4865664
id: db76c9a0-b4fc-4aa2-8afb-47cd78ef7138
terms: check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, current=e2533580-1486-4dc9-8697-e6c53605fe72, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, finding=ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, move=d904dc0d-231f-4e93-ac74-7babe888107d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d

Problem: a hooksPath set away from the tool's directory leaves every hook file in place and current while none of them runs.
Standard: a tool that depends on a setting verifies the setting, not only the files behind it.

- A) Yes.
  Why: a repointed hooksPath is a finding wherever check runs, by hand or in CI, with install as the move.
  Costs: check reads git's configuration; a fresh clone and a CI checkout have no hooksPath until install runs there.
- B) No.
  Why: check judges files; git's configuration is the owner's machine.
  Costs: a manager that repoints hooksPath silences the tool's hooks, and only a commit that should have failed shows it.

**Answer:** A
