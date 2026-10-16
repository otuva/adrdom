# Security

Report a vulnerability through GitHub's private reporting on this repository ("Report a vulnerability" under the Security tab), not in an issue.

## What the tool defends

- Owner commands (`approve`, `answer`, `unban`, `serve`, `uninstall`) run only at an interactive terminal. This stops an agent that runs one because it looks helpful, not one that fakes a terminal to pass as the owner. Against that, only a separate account or machine for the owner holds.
- `adr serve` binds to 127.0.0.1 and takes a token the terminal prints. A process on the same machine that reads that token is the owner.
- The ban list holds SHA-256 hashes of banned words, never the words. A short or guessable word still falls to a dictionary.
- The ledger is append-only and checked at every commit and push, and in CI. It is a text file; the hooks notice an edit, they do not prevent one.
