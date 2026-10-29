# model-switch-policy
change: classifier.go classifyPrompt now outputs topic: same|new, size: small|large, task. pickModel(cur,topic,size): same->stay cur; new+small->sonnet; new+large->opus. parseRoute(out,cur) requires topic+size+task (else classifier failure -> line sent as-is, no switch).
route(): unchanged; rotates only when model != r.a.Model. Classifier input adds "keep it unless topic is new".
test: conductor_test.go TestMainRouteSwitchPolicy (same small keeps opus; new small->sonnet; new large->opus), TestParseRoute updated. Live test (CHAUDE_LIVE) not run; want for small cases now "opus,sonnet".
not done: Opus->Sonnet delegation (agent) left to existing delegation rule. LLM same/new judgment unverified live.
no commit.
