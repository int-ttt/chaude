//! Esc Esc rewind picker key and draw (Go tui.go key() rewind branch; tui.go rewind draw).
//! The model (prompt list, selection, cut) is core `chaude_core::rewind`; this file only reads keys and draws rows.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::bar::clip;
use super::render::{DIM_ON, FG_OFF, ORANGE_ON, RESET, trunc_ellipsis};
use chaude_core::rewind::Cut;
pub use chaude_core::rewind::{NOTHING, REWOUND, Rewind};

// Go rewind draw: header line.
pub const HEADER: &str = "Rewind to a prompt  (↑/↓ select · enter rewind · esc cancel)";

// Go tui.go key() rewind branch: up/k, down/j move; esc, ctrl+c, q cancel; enter confirms.
// While open every key is consumed by the picker. Returns the cut only on enter.
pub fn key(rw: &mut Rewind, key: KeyEvent, blocks: &[(&str, &str)]) -> Option<Cut> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => rw.move_by(-1),
        KeyCode::Down | KeyCode::Char('j') => rw.move_by(1),
        KeyCode::Esc => rw.cancel(),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => rw.cancel(),
        KeyCode::Char('q') => rw.cancel(),
        KeyCode::Enter => return rw.confirm(blocks),
        _ => {}
    }
    None
}

// Go rewind draw: header, then up to `body_h - 1` prompt rows around the selection (newest last).
// Row text is "%2d) <prompt>" with newlines folded and the prompt cut at 100 chars; the selected row is
// orange with "> ", the others dim with "  ".
pub fn lines(rw: &Rewind, blocks: &[(&str, &str)], width: usize, body_h: usize) -> Vec<String> {
    let mut out = vec![format!("{ORANGE_ON}{}{FG_OFF}", clip(HEADER, width))];
    let turns = rw.turns();
    let n = turns.len() as isize;
    let h = body_h as isize;
    let from = (rw.sel() as isize - h / 2).min(n - (h - 2)).max(0);
    let mut i = from;
    while i < n && out.len() + 1 < body_h {
        let idx = i as usize;
        let text: String = blocks[turns[idx]]
            .1
            .replace('\n', " ")
            .chars()
            .take(100)
            .collect();
        let row = format!("{:>2}) {}", idx + 1, text);
        let row = if idx == rw.sel() {
            format!(
                "{ORANGE_ON}> {}{FG_OFF}",
                trunc_ellipsis(&row, width.saturating_sub(2))
            )
        } else {
            format!(
                "{DIM_ON}  {}{RESET}",
                trunc_ellipsis(&row, width.saturating_sub(2))
            )
        };
        out.push(row);
        i += 1;
    }
    while out.len() < body_h {
        out.push(String::new());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bl() -> Vec<(&'static str, &'static str)> {
        vec![
            ("user", "/help"),
            ("note", "n"),
            ("user", "one"),
            ("asst", "r1"),
            ("user", "two"),
            ("asst", "r2"),
        ]
    }

    #[test]
    fn keys_move_confirm_and_cancel() {
        let b = bl();
        let mut s = Rewind::default();
        s.open(&b);
        let press = |c| KeyEvent::new(c, KeyModifiers::NONE);
        assert_eq!(key(&mut s, press(KeyCode::Up), &b), None);
        assert_eq!(s.sel(), 0);
        assert_eq!(key(&mut s, press(KeyCode::Char('j')), &b), None);
        assert_eq!(s.sel(), 1);
        let got = key(&mut s, press(KeyCode::Enter), &b);
        assert_eq!(got.map(|c| c.cut), Some(4));
        s.open(&b);
        key(&mut s, press(KeyCode::Esc), &b);
        assert!(!s.is_open());
        s.open(&b);
        key(
            &mut s,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            &b,
        );
        assert!(!s.is_open());
    }

    #[test]
    fn draw_fills_body_and_marks_selection() {
        let b = bl();
        let mut s = Rewind::default();
        s.open(&b);
        let rows = lines(&s, &b, 80, 4);
        assert_eq!(rows.len(), 4);
        assert!(rows[0].contains(HEADER));
        assert!(rows[2].starts_with(ORANGE_ON));
        assert!(rows[2].contains(" 2) two"));
        assert!(rows[1].contains(" 1) one"));
    }

    // Go rewind draw at the 100x30 transcript size: the window follows the selection, the header is row 0,
    // the last row is left blank, and a multi-line prompt is folded to one line and cut at 100 chars.
    #[test]
    fn draw_100x30_window_follows_selection() {
        let long = "x".repeat(150);
        let mut b: Vec<(&str, String)> = (1..=40).map(|i| ("user", format!("p{i}"))).collect();
        b.push(("user", "a\nb".to_string()));
        b.push(("user", long));
        let refs: Vec<(&str, &str)> = b.iter().map(|(k, t)| (*k, t.as_str())).collect();
        let mut s = Rewind::default();
        assert!(s.open(&refs));
        assert_eq!(s.sel(), 41, "newest prompt is selected");
        let rows = lines(&s, &refs, 100, 30);
        assert_eq!(rows.len(), 30);
        assert!(rows[0].contains(HEADER));
        assert!(rows[29].is_empty(), "one blank row under the list");
        // from = min(41 - 15, 42 - 28) = 14: rows 1..=28 show prompts 15..=42, row 29 is blank.
        assert!(rows[1].contains("15) p15"), "{:?}", rows[1]);
        assert!(rows[27].contains("41) a b"), "{:?}", rows[27]);
        assert!(rows[28].starts_with(ORANGE_ON));
        assert!(rows[28].contains("42) "));
        // Go ansiTrunc(t, m.w): "> 42) " takes 6 columns, so 93 x's fit and the row ends with "…" at width 100.
        assert!(rows[28].contains(&format!("{}…", "x".repeat(93))));
        assert!(!rows[28].contains(&"x".repeat(94)));
        // The prompt itself is cut at 100 chars (Go trunc(text, 100)) when the width allows it.
        let wide = lines(&s, &refs, 200, 30);
        assert!(wide[28].contains(&"x".repeat(100)));
        assert!(!wide[28].contains(&"x".repeat(101)));
        // The earlier selection moves the window back to the top of the list.
        s.move_by(-41);
        let rows = lines(&s, &refs, 100, 30);
        assert!(rows[1].contains(" 1) p1"), "{:?}", rows[1]);
    }
}
