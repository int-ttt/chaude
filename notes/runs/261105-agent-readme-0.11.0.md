# readme-0.11.0 run
status: done (not committed)
changed: README.md (rewritten, section order per brief); README.ko.md (same structure, 존댓말)
sections: What is it | Features | Installation (macOS/Linux, Windows, From source) | Quick start | Usage (REPL keys, Permission modes, Slash commands, Command line, Updating) | Configuration | How it works (agents table, rotation, cost, Claude Code sessions) | Documentation | Upgrading from Go | Contributing (Releasing) | License
removed from top: Go deprecation blockquote (moved to Upgrading from Go); Effect/FAQ/Updating/Releasing headings folded into sections above
added: From source (cargo), Configuration (flags, env vars from install.sh/ps1), Documentation link plan/rust-port.md, Upgrading from Go (installers remove old ~/.local/bin install)
fixed: Installation badge anchor #install -> #installation (README.md); blank line before Contributing
kept facts: all benchmark numbers, rotation thresholds, key table, slash commands, permission modes, agent table, release steps
unverified: default permission mode at startup (source: Config mode = Default in api.rs:1061; old README said auto) -> not stated; --cwd/--show-rotation/--metrics-dir defaults -> see chaude repl -h; "(0.9.14)" note kept as-is
