# TUI parity big-gap list (261104)
source: plan/tui-parity.md audit, run 261104-haiku-tui-parity-check
gap: L06 L07 L11 panel and background rows. Rust has no panel model (bar.rs two-row top bar, mod.rs body/chrome). Go panelLines/bgLines/queueLines not ported.
gap: L08 small-height no-overflow for panel/bg/queue. Depends on the L06/L07/L11 gap.
gap: R14 ellipsis truncation. trunc_ellipsis (render.rs:317) has no production caller.
gap: I26 rewind picker Open/Move/Cancel/Confirm. Not audited this run (rewind.rs:36-98 exists).
gap: I05 premise differs. Go and Rust both quit on Ctrl+D; delete only on Delete key. Plan wording needs update, not code.
core-dependent: B21-B23 queue semantics, I18 mode set, R16/R17 context numbers, R07 permission source. Core owns these.
not audited this run: L01-L05, L09-L10, L12-L14, B*, C*, E*, X*, A*, S*, I26 (144 items still [ ]).
