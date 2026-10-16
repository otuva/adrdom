# vhs docs/demo.tape   (from the repository root, as the owner, at a terminal)
Output docs/demo.gif
Set Shell bash
Set FontSize 15
Set Width 1100
Set Height 600
Set Theme "Catppuccin Mocha"
Set TypingSpeed 40ms

Hide
Type "unset CLAUDE_CODE_SESSION_ID; cd $(sh docs/demo-setup.sh /tmp) && clear"
Enter
Sleep 1s
Show

Type "adr show"
Enter
Sleep 3s
Type "adr approve adrdom/terms.md retry"
Enter
Sleep 3s
Type "adr plan adrdom/decisions/http.md"
Enter
Sleep 12s
Type "adr approve adrdom/decisions/http.md"
Enter
Sleep 3s
Type "adr ask adrdom/decisions/http.md"
Enter
Sleep 2s

Hide
Type ". .q; tput cuu1; tput el"
Enter
Show
Type "adr answer $Q A"
Enter
Sleep 2s
Type "git add adrdom && git commit -qm 'decide: the client retries' && clear"
Enter
Sleep 1s

Type "# the builder, in another session: a test that names the decision"
Enter
Type "mkdir -p tests && printf '# adr: %s\n' $ID > tests/retry_test.sh && git add tests && git commit -qm 'test: retry'"
Enter
Sleep 1s
Type "adr show"
Enter
Sleep 3s
Type "clear"
Enter

Type "# later, an agent changes its mind about an answered question"
Enter
Type "sed -i 's/^- A) Yes./- A) Yes, three times./' adrdom/decisions/http.md"
Enter
Type "git commit -am 'retry three times'"
Enter
Sleep 5s
