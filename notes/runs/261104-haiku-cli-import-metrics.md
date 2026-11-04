# 261104 haiku cli import metrics

task: cli items from core b (metrics 7b60fb4/cf3d7f1, import, record modules)
branch: rust-port
scope: crates/cli only (no core edits, no cargo fmt on chaude-core)

done:
- chaude metrics [--dir D] [--csv]: commit 37daa8d. core metrics_report::summary/csv. dir auto = record::dir_for(cwd)/metrics. bad arg or missing --dir value -> exit 2.
- /import [id|latest] view: commit e52e14d. core import_claude::load_import_arg -> render::import_blocks(imp.view) via record_block. dead_code allow removed.
- /import replaces the view (Go importedMsg -> m.blocks = importBlocks): mod.rs import_view clears transcript, wide, prompts before rendering. test import_replaces_the_transcript_view (user+assistant jsonl fixture). commit: see git log (fix commit after ff01bee).
- records dir/folder/resume to core: commit 9c6832b. record_dir -> record::dir_for; folder_name -> record_folder::folder_name; /resume -> record_turn::resume (records::list_date).
- note: commands.rs needed no change (Dispatch::Resume already passes raw args).

checks:
- cargo build -p chaude-cli -> exit 0
- cargo test -p chaude-cli -> 184 passed, 0 failed (includes import_replaces_the_transcript_view)
- cargo clippy -p chaude-cli --no-deps -- -D warnings -> exit 0
- cargo fmt -p chaude-cli -- --check -> exit 0
- smoke: HARNESS_RECORD_DIR=/tmp/nonexist-rec chaude metrics -> summary, exit 0; --csv -> 14-col header, exit 0

blocked:
- /import install of the snapshot into the next prompt: needs core api.rs public install API (pending_snapshot private). TUI shows a note saying so.

doubts:
- resume error text: core record_turn.rs:168 uses {arg:?}. Go record_turn.go:90 uses fmt.Errorf("no project matches %q"), so Go also quotes. Same output for plain ASCII; differs only for quotes/escapes/non-printables. Kept as-is (matching Go). Coordinator asked for plain text; not changed without confirmation. Core file not mine to edit.
- cli pick/list_projects/load_project/filter_projects kept: RecordProject/RecordEntry types drive RecPicker and load_record. Core equivalents exist (record_folder, record::parse_entries) if the picker moves to core types.
- rotate CSV handoff_valid formatting for non-bool values: unchanged (core note).
