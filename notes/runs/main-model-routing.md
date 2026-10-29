# main-model-routing
existing: --conductor (default on) one-shot Sonnet classifier per prompt (classifier.go), route() switches session model (restart if no history else rotate w/ handoff). Reused.
change: classifyPrompt now size-based, outputs sonnet|opus (small->sonnet; large/multi-file/design/tradeoff/pushback/correction/strategy->opus); parseRoute maps haiku->sonnet (main never haiku).
unchanged: acks + slash cmds stay on current session; /mode conductor-off disables; scroll `held` untouched.
test: conductor_test.go TestMainRouteSizeModels (+TestParseRoute adjusted). TestClassifierLive (CHAUDE_LIVE=1) not rerun.
uncertain: LLM size judgment unverified live; switching rotates session (handoff cost) on each model flip; haiku-expecting live test may need update.
no commit.
