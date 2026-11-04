# 261104 haiku cli import metrics

task: cli items from core b (metrics 7b60fb4/cf3d7f1, import, record modules)
branch: rust-port
scope: crates/cli only (no core edits, no cargo fmt on chaude-core)

done:
- chaude metrics [--dir D] [--csv]: commit 37daa8d. core metrics_report::summary/csv. dir auto = record::dir_for(cwd)/metrics. bad arg or missing --dir value -> exit 2.
- /import [id|latest] view: commit e52e14d. core import_claude::load_import_arg -> render::import_blocks(imp.view) appended via record_block. dead_code allow removed.
- records dir/folder/resume to core: commit 9c6832b. record_dir -> record::dir_for; folder_name -> record_folder::folder_name; /resume -> record_turn::resume (records::list_date).
- note: commands.rs needed no change (Dispatch::Resume already passes raw args).

checks:
- cargo build -p chaude-cli -> exit 0
- cargo test -p chaude-cli -> 183 passed, 0 failed
- cargo clippy -p chaude-cli --no-deps -- -D warnings -> exit 0
- cargo fmt -p chaude-cli -- --check -> exit 0
- smoke: HARNESS_RECORD_DIR=/tmp/nonexist-rec chaude metrics -> summary, exit 0; --csv -> 14-col header, exit 0

blocked:
- /import install of the snapshot into the next prompt: needs core api.rs public install API (pending_snapshot private). TUI shows a note saying so.

doubts:
- /import view appends to the transcript; Go importedMsg replaces blocks. Not matched on purpose (wide-row indices).
- resume error text: core uses {arg:?} debug quoting, cli used "{arg}". Same for plain ASCII args; differs only for quotes/escapes.
- cli pick/list_projects/load_project/filter_projects kept: RecordProject/RecordEntry types drive RecPicker and load_record. Core equivalents exist (record_folder, record::parse_entries) if the picker moves to core types.
- rotate CSV handoff_valid formatting for non-bool values: unchanged (core note).
