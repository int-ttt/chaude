# agent view + usage popup
agent view open: left-click a panel row (fg/bg) OR ctrl+t (first running agent, else newest); tab/shift+tab (left/right) switch agent; up/down/pgup/pgdn/wheel scroll; esc/q/ctrl+c close
read-only: avKey drops every other key (input box untouched, nothing queued/sent)
data source: stream events with parent_tool_use_id==Agent call id -> agentRec.Log (text, "● Tool(arg)", "⎿ result"), cap 400 lines; captured in runTurn handle + idleEvent (bg) via tracker.onSubEvent
usage popup: /usage output -> usageMsg -> centered bordered box over transcript area; esc/q/enter/ctrl+c close; input swallowed. Passthrough (repl.go) sends usageMsg instead of textMsg
files: agentview.go (new), agentview_test.go (new), tui.go (fields, Update/key/View hooks, rowsA), drain.go (Log), repl.go (2 onSubEvent calls, usage send)
cursor math: untouched (overlays replace viewport content only; same line counts)
limits: click row = vp.Height+popH+queueH+inH+2 (assumes no wrap); popup blanks transcript behind it (no true transparency); sub-agent events visible only if claude stream emits them with parent_tool_use_id (unverified live); finished agents not in panel -> reach via ctrl+t/tab; overlay closes on permission prompt; popup not drawn if /usage returns no text
verify: go build, go vet, go test -count=1 -race ./internal/harness ok
