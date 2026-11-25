// Host compatibility for the TUI, mainly JetBrains (JediTerm) on Windows: Enter can arrive as a raw '\r' char, the
// console may run a legacy code page, JetBrains fonts lack some glyphs, and a Run console has no terminal at all.
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use unicode_width::UnicodeWidthChar;

// Press and Repeat are input. Release is dropped: Windows sends a Release for every key as well.
pub fn accepts(kind: KeyEventKind) -> bool {
    kind != KeyEventKind::Release
}

// Enter as some hosts send it: a raw '\r' or '\n' char, or Ctrl+M (the same byte as '\r'). Becomes KeyCode::Enter.
// Ctrl+J is left alone: it is the newline key (see on_key).
pub fn normalize_enter(mut key: KeyEvent) -> KeyEvent {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let is_enter = match key.code {
        KeyCode::Char('\r' | '\n') => true,
        KeyCode::Char('m') => ctrl,
        _ => false,
    };
    if is_enter {
        key.code = KeyCode::Enter;
        key.modifiers.remove(KeyModifiers::CONTROL);
    }
    key
}

// ASCII glyphs on under JetBrains (its font lacks box and symbol glyphs) or with CHAUDE_ASCII=1.
pub fn ascii_glyphs(emulator: Option<&str>, flag: Option<&str>) -> bool {
    flag.map(str::trim) == Some("1") || emulator.is_some_and(|e| e.contains("JetBrains"))
}

// Replaces each special glyph with one ASCII char, padded with spaces to the glyph's display width so that the
// columns computed by unicode-width still line up. Korean and other text is left as it is.
pub fn asciify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let Some(rep) = ascii_of(c) else {
            out.push(c);
            continue;
        };
        out.push_str(rep);
        let width = UnicodeWidthChar::width(c).unwrap_or(1);
        for _ in rep.chars().count()..width {
            out.push(' ');
        }
    }
    out
}

fn ascii_of(c: char) -> Option<&'static str> {
    Some(match c {
        '─' | '━' | '═' | '╌' | '╍' | '┄' | '┈' | '—' | '–' => "-",
        '│' | '┃' | '║' | '╎' | '┆' | '┊' | '╏' | '┇' | '┋' => "|",
        '┌' | '┐' | '└' | '┘' | '├' | '┤' | '┬' | '┴' | '┼' | '╭' | '╮' | '╰' | '╯' => {
            "+"
        }
        '┏' | '┓' | '┗' | '┛' | '┣' | '┫' | '┳' | '┻' | '╋' | '╔' | '╗' | '╚' | '╝' | '╠' | '╣'
        | '╦' | '╩' => "+",
        '╬' | '╒' | '╕' | '╘' | '╛' | '╞' | '╡' | '╤' | '╧' | '╪' | '╓' | '╖' | '╙' | '╜' | '╟'
        | '╢' | '╥' => "+",
        '╨' | '╫' | '╳' => "+",
        '█' | '▓' => "#",
        '▒' => "%",
        '░' => ".",
        '▌' | '▐' | '▍' | '▎' | '▏' | '▋' | '▊' | '▉' | '▕' => "|",
        '▀' | '▄' => "=",
        '●' | '⏺' | '◉' | '⦿' | '◆' | '◈' | '•' | '■' | '▪' | '⏹' => "*",
        '○' | '◯' | '◇' | '◦' | '□' | '▫' => "o",
        '✓' | '✔' | '√' => "v",
        '✗' | '✘' | '✕' | '✖' | '×' => "x",
        '⎿' => "L",
        '⏵' | '▶' | '►' | '▸' | '❯' | '›' | '➜' | '→' | '⟶' | '➔' => ">",
        '◀' | '◂' | '◄' | '‹' | '←' => "<",
        '↑' => "^",
        '↓' => "v",
        '…' | '·' | '∙' => ".",
        '“' | '”' => "\"",
        '‘' | '’' => "'",
        '⚠' => "!",
        '\u{FE0F}' => "",
        _ if matches!(c as u32, 0x1F000..=0x1FAFF | 0x2600..=0x27BF) => "?",
        _ => return None,
    })
}

// Message for an IDE Run console: no TTY on stdin or stdout, so the TUI cannot draw or read keys.
pub const NO_TTY_MESSAGE: &str = "chaude: the interactive UI needs a terminal, but this process has no TTY \
(an IDE Run console). Enable \"Emulate terminal in output console\" in the Run configuration, or run chaude \
from the IDE terminal. HARNESS_PLAIN=1 selects line mode instead.";

// Windows only: UTF-8 code pages and VT processing before the TUI starts, and the old values back on exit.
pub fn prepare_console() {
    #[cfg(windows)]
    console::prepare();
}

pub fn restore_console() {
    #[cfg(windows)]
    console::restore();
}

#[cfg(windows)]
mod console {
    use std::sync::atomic::{AtomicU32, Ordering};

    type Handle = isize;
    const STD_OUTPUT_HANDLE: u32 = (-11i32) as u32;
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
    const CP_UTF8: u32 = 65001;

    // Values before chaude changed them; 0 means unchanged or no console. restore() swaps them out, so it is idempotent.
    static OLD_IN_CP: AtomicU32 = AtomicU32::new(0);
    static OLD_OUT_CP: AtomicU32 = AtomicU32::new(0);
    static OLD_OUT_MODE: AtomicU32 = AtomicU32::new(0);

    // SAFETY (all calls): plain kernel32 console APIs; each takes scalar args or a valid pointer to a local.
    unsafe extern "system" {
        fn GetStdHandle(n: u32) -> Handle;
        fn GetConsoleMode(h: Handle, mode: *mut u32) -> i32;
        fn SetConsoleMode(h: Handle, mode: u32) -> i32;
        fn GetConsoleCP() -> u32;
        fn GetConsoleOutputCP() -> u32;
        fn SetConsoleCP(cp: u32) -> i32;
        fn SetConsoleOutputCP(cp: u32) -> i32;
    }

    pub fn prepare() {
        unsafe {
            OLD_IN_CP.store(GetConsoleCP(), Ordering::SeqCst);
            OLD_OUT_CP.store(GetConsoleOutputCP(), Ordering::SeqCst);
            SetConsoleCP(CP_UTF8);
            SetConsoleOutputCP(CP_UTF8);
            let h = GetStdHandle(STD_OUTPUT_HANDLE);
            let mut mode = 0u32;
            if GetConsoleMode(h, &mut mode) != 0 {
                OLD_OUT_MODE.store(mode, Ordering::SeqCst);
                SetConsoleMode(h, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
            }
        }
    }

    pub fn restore() {
        let in_cp = OLD_IN_CP.swap(0, Ordering::SeqCst);
        let out_cp = OLD_OUT_CP.swap(0, Ordering::SeqCst);
        let mode = OLD_OUT_MODE.swap(0, Ordering::SeqCst);
        unsafe {
            if in_cp != 0 {
                SetConsoleCP(in_cp);
            }
            if out_cp != 0 {
                SetConsoleOutputCP(out_cp);
            }
            if mode != 0 {
                SetConsoleMode(GetStdHandle(STD_OUTPUT_HANDLE), mode);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEventState;
    use unicode_width::UnicodeWidthStr;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn enter_forms_map_to_enter() {
        let none = KeyModifiers::NONE;
        assert_eq!(
            normalize_enter(key(KeyCode::Enter, none)).code,
            KeyCode::Enter
        );
        assert_eq!(
            normalize_enter(key(KeyCode::Char('\r'), none)).code,
            KeyCode::Enter
        );
        assert_eq!(
            normalize_enter(key(KeyCode::Char('\n'), none)).code,
            KeyCode::Enter
        );
        let m = normalize_enter(key(KeyCode::Char('m'), KeyModifiers::CONTROL));
        assert_eq!(m.code, KeyCode::Enter);
        assert!(!m.modifiers.contains(KeyModifiers::CONTROL));
    }

    #[test]
    fn other_keys_are_untouched() {
        let k = key(KeyCode::Char('m'), KeyModifiers::NONE);
        assert_eq!(normalize_enter(k), k);
        let j = key(KeyCode::Char('j'), KeyModifiers::CONTROL);
        assert_eq!(normalize_enter(j), j);
        let a = key(KeyCode::Char('a'), KeyModifiers::ALT);
        assert_eq!(normalize_enter(a), a);
    }

    #[test]
    fn release_is_dropped_press_and_repeat_kept() {
        assert!(accepts(KeyEventKind::Press));
        assert!(accepts(KeyEventKind::Repeat));
        assert!(!accepts(KeyEventKind::Release));
    }

    #[test]
    fn ascii_flag_from_emulator_or_env() {
        assert!(ascii_glyphs(Some("JetBrains-JediTerm"), None));
        assert!(ascii_glyphs(None, Some("1")));
        assert!(ascii_glyphs(None, Some(" 1 ")));
        assert!(!ascii_glyphs(None, Some("0")));
        assert!(!ascii_glyphs(Some("iTerm.app"), None));
        assert!(!ascii_glyphs(None, None));
    }

    #[test]
    fn asciify_maps_box_and_symbols_keeping_width() {
        let src = "╭─ ● ✓ ✗ ⎿ █░ …";
        let out = asciify(src);
        assert_eq!(out, "+- * v x L #. .");
        assert!(out.is_ascii());
        assert_eq!(out.width(), src.width());
    }

    #[test]
    fn asciify_keeps_korean_and_pads_emoji() {
        let src = "한글 │ 😀 ⏺";
        let out = asciify(src);
        assert!(out.starts_with("한글 | ? "));
        assert!(out.is_char_boundary(out.len()));
        assert_eq!(out.width(), src.width());
    }
}
