#!/bin/sh
# Builds the throwaway repository docs/demo.tape records in. Prints its path.
set -e
unset CLAUDE_CODE_SESSION_ID
D="${1:-$(mktemp -d)}/adr-demo"
rm -rf "$D"; mkdir -p "$D"; cd "$D"
git init -q; git config user.name demo; git config user.email demo@example.com
git commit -q --allow-empty -m init
adr install --claude >/dev/null
git add -A; git commit -qm 'adr install'
printf '# Terms\n\n## retry\ndefinition: One more attempt at a request that failed.\naliases: retries\n' > adrdom/terms.md
mkdir -p adrdom/decisions
cat > adrdom/decisions/http.md <<'MD'
# Http

What the client does when a request fails.

### Does the client retry a failed request?
depends: retry

Problem: a request that fails once fails the whole job, though the next attempt would pass.
Standard: retry with backoff, bounded.

- A) Yes.
  Why: a transient failure costs one more request, not the job.
  Costs: a request that is not idempotent may run twice.
- B) No.
  Why: every failure is seen once, where it happened.
  Costs: every transient failure fails the job.
MD
adr plan adrdom/decisions/http.md >/dev/null 2>&1 || true
printf '%s\n' 'Q=$(adr show adrdom/decisions/http.md | grep -oE "[0-9a-f-]{36}@[0-9a-f]+")' 'ID=${Q%@*}' > .q
echo "$D"
