# 261104 haiku go view+resume fix (0.10.1 workstream, partial)
scope: deprecated/src/main/intt/internal/harness only
done:
- resume: launch agentId kept (agentRec.AgentID, savedAgent.agent_id); SendMessage resumedAgentId success:true reactivates agent (drain.go resumeLocked); test TestResumeSendMessageReactivatesAgent PASS
- /view: rows and prompt head show agent name (Session.Role; sessionInfo.Agent; agentLabel), not session id
- /view level 2: up/down/pgup/pgdn/home/end + wheel scroll prompt (svScroll clamp); left/right prompt step; esc back
- gofmt: resume.go, sessview.go, agentstore.go formatted (formatting only)
- sessview_test.go: svModel gives sessaaa1 Role coder; TestViewEscBacksOutToNormalScreen now expects "coder", not id (stale id assertion); new TestViewPromptScrollsInsideTranscript
checks:
- gofmt -l . (deprecated/src/main/intt): clean
- go vet ./... : PASS (exit 0)
- go test ./... -count=1: PASS (clean, harness, scripts, settings, update ok)
- go test ./internal/harness -run TestView -v: 7 TestView* PASS
pending (not done):
- commit: see git log for hash (Go paths only)
- release 0.10.1: NOT done. reason: scripts/bump-version.sh runs cargo update --workspace which rewrites shared Cargo.lock while crates coders are active; also touches Cargo.toml
- CHANGELOG / README.md / README.ko.md 0.10.1 entries: not written
- push, tag v0.10.1, gh run watch, release assets: not done
- Mac deploy: BLOCKED. memory feedback_release_deploy.md: deploy target unconfirmed, ask user before pushing deploy target
doubts:
- earlier-run sessions (prevOwned, no Agent) show transcript title or "session" instead of agent name
