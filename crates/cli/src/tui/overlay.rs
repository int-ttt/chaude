// Floating panels drawn above the input line. Pure functions: the caller decides the size and places the rows.
// The slash completion menu follows tui.go (suggestions, max 8 rows, the selected row in orange).

use super::bar::clip;
use super::commands::SlashCmd;
use super::render::{FG_OFF, GREY_ON, ORANGE_ON};

// L09: the slash suggestion popup shows at most this many rows.
pub const SLASH_MAX_ROWS: usize = 8;

// Rows of the slash menu for `items`, starting at a window that keeps `sel` visible. `max` caps the row
// count (the caller passes what fits). Each row is clipped to `width` columns.
// tui.go:1262-1279: "  %-9s %s" per row; the selected row is orange, the others grey.
pub fn slash_popup(items: &[&SlashCmd], sel: usize, width: usize, max: usize) -> Vec<String> {
    let n = items.len().min(max).min(SLASH_MAX_ROWS);
    if n == 0 {
        return Vec::new();
    }
    let sel = sel.min(items.len() - 1);
    let start = if sel >= n { sel + 1 - n } else { 0 };
    items[start..start + n]
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let on = if start + i == sel { ORANGE_ON } else { GREY_ON };
            let text = clip(&format!("  {:<9} {}", c.name, c.desc), width);
            format!("{on}{text}{FG_OFF}")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::commands::{SLASH_CMDS, suggestions};
    use super::*;
    use unicode_width::UnicodeWidthStr;

    // The row text without its SGR color codes.
    fn plain(row: &str) -> String {
        let mut out = String::new();
        let mut in_esc = false;
        for c in row.chars() {
            match (in_esc, c) {
                (false, '\x1b') => in_esc = true,
                (true, 'm') => in_esc = false,
                (true, _) => {}
                _ => out.push(c),
            }
        }
        out
    }

    #[test]
    fn popup_lists_matches_and_colors_the_selected_row() {
        let sg = suggestions("/mo", false, false);
        let rows = slash_popup(&sg, 1, 80, 8);
        assert_eq!(rows.len(), sg.len());
        assert!(rows[0].starts_with(GREY_ON));
        assert!(rows[1].starts_with(ORANGE_ON));
        assert_eq!(
            plain(&rows[0]),
            "  /model    show or switch model: /model <name>"
        );
        assert_eq!(plain(&rows[1]), format!("  /mode     {}", sg[1].desc));
    }

    #[test]
    fn popup_window_follows_the_selection_and_respects_the_cap() {
        let all: Vec<&SlashCmd> = SLASH_CMDS.iter().collect();
        let rows = slash_popup(&all, all.len() - 1, 80, 5);
        assert_eq!(rows.len(), 5);
        assert!(rows[4].starts_with(ORANGE_ON));
        assert!(plain(&rows[4]).starts_with("  /quit "));
        assert!(slash_popup(&all, 0, 80, 0).is_empty());
        assert_eq!(slash_popup(&all, 0, 80, 100).len(), SLASH_MAX_ROWS);
    }

    #[test]
    fn popup_rows_never_exceed_the_width() {
        let all: Vec<&SlashCmd> = SLASH_CMDS.iter().collect();
        for r in slash_popup(&all, 0, 12, 8) {
            assert!(UnicodeWidthStr::width(plain(&r).as_str()) <= 12);
        }
    }
}
