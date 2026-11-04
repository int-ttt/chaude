kind: api-doc
topic: tui overlays (rewind picker, agent view)
owner: Rust TUI B (crates/cli/src/tui/rewind.rs, agentview.rs)
mod.rs: not edited by TUI B. TUI A must add the hook calls below.

## rewind.rs
Port of Go history.go rewindState + tui.go rewind key branch and draw.
Pure over `(kind, text)` pairs. The caller owns the transcript and applies the cut.

- `pub struct Rewind` (Default). Methods:
  - `is_open(&self) -> bool`
  - `open(&mut self, blocks: &[(&str, &str)]) -> bool`
    lists "user" blocks with non-empty text not starting with '/'. Returns false when empty. On false, show `rewind::NOTHING`.
  - `move_by(&mut self, d: isize)` wraps around.
  - `cancel(&mut self)`
  - `confirm(&mut self, blocks) -> Option<Cut>` closes the picker.
  - `key(&mut self, KeyEvent, blocks) -> Option<Cut>`
    up/k, down/j move; esc, ctrl+c, q cancel; enter confirms. Every key is consumed while open. Returns Some(Cut) only on enter.
  - `lines(&self, blocks, width: usize, body_h: usize) -> Vec<String>`
    exactly `body_h` rows. Row 0 is the header, the last row is blank when there is room.
- `pub struct Cut { pub cut: usize, pub text: String }`
  Keep `blocks[..cut]`. Put `text` back in the input. Show `rewind::REWOUND` as a note.
- Consts: `NOTHING`, `REWOUND`, `HEADER`.

Blocks are `(kind, text)` pairs. The Go source uses `block{kind, text, ...}`. Map the Rust transcript to `(&str, &str)` before calling.

## agentview.rs
Port of Go agentview.go (agentViewLines, avKey, cycleAgent, openAgentKey, onSubEvent, sortedAll, maxAgentLog=400).
Title shows the agent name, not the id.

- `pub struct AgentRec` (Default, Clone). Fields:
  `id`, `task`, `prompt`, `model` (empty inherits session model), `name` (shown), `failed`, `done`, `interrupted`, `started: bool` (false = queued), `started_at: u64` (ms, sort key), `log: Vec<String>`.
  - `state() -> AgentState`
  - `status_text(&self, session_model: &str) -> String`
- `pub fn sort_all(recs: &mut [AgentRec])`
- `pub fn sub_event_lines(ev: &serde_json::Value) -> Option<(String, Vec<String>)>`
  Go onSubEvent formatting. Returns (agent id, lines to append).
- `pub fn push_log(log: &mut Vec<String>, add: Vec<String>)`
  keeps the newest `MAX_AGENT_LOG` (400) lines.
- `pub struct AgentView { pub off: usize, .. }` (open with `AgentView::open(id)`)
  - `open(id: &str) -> Self`
  - `open_first(recs: &[AgentRec]) -> Option<Self>`
  - `cycle(&mut self, recs: &[AgentRec], d: isize)` (tab / shift-tab over sorted agents, wraps)
  - `key(&mut self, KeyEvent, recs: &[AgentRec], vp_h: usize) -> Action`
    `Action::None` or `Action::Close`. Read-only: typing is dropped. esc and q close. Up/down/pgup/pgdn scroll.
  - `lines(&mut self, rec: &AgentRec, session_model: &str, width: usize, hh: usize) -> Vec<String>`
    exactly `hh` rows. Clamps `off` to the body.
- Consts: `MAX_AGENT_LOG`, `NO_AGENTS` ("no agents yet"). Head "Agent (read-only)" and hint are in the module.

Known gap: `wrap()` hard-wraps by display column. Go lipgloss Width does word wrap.

## mod.rs hook calls for TUI A (not added by TUI B)
Add to crates/cli/src/tui/mod.rs. Field names are suggestions.

1. State: `rw: rewind::Rewind` and `av: Option<agentview::AgentView>` on the app struct.
2. Esc Esc on an empty input (Go tui.go ~line 709):
   `if !self.rw.open(&refs) { push note rewind::NOTHING }`
   where `refs: Vec<(&str, &str)>` is built from the transcript blocks.
3. While `self.rw.is_open()`, route every key first:
   `if let Some(cut) = self.rw.key(key, &refs) { truncate transcript to cut.cut; set input to cut.text; push note rewind::REWOUND }`
4. Draw when `self.rw.is_open()`:
   `self.rw.lines(&refs, width, body_h)` replaces the transcript area.
5. Agent view:
   open on ctrl+t: `self.av = AgentView::open_first(&recs)`. Use `AgentView::open(id)` for a specific agent.
   While `self.av.is_some()`, route keys to `key(key, &recs, vp_h)`. On `Action::Close`, set `self.av = None`.
   Draw with `lines(&rec, session_model, width, hh)`, where `rec` is a clone of the tracked agent.
6. Keep `recs` sorted with `sort_all` before cycling.
7. Sub-agent events: call `sub_event_lines(&ev)` and `push_log(&mut rec.log, lines)`.

## verification
- `cargo test -p chaude-cli rewind`
- `cargo test -p chaude-cli agentview`
- 100x30 draw tests: `rewind::tests::draw_100x30_window_follows_selection`, `agentview::tests::draw_100x30_shows_name_not_id_and_clamps_scroll`
