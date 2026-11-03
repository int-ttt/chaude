// Floating panels drawn above the input line. Pure functions: the caller decides the size and places the rows.
// The slash completion menu follows tui.go (suggestions, max 8 rows, the selected row marked).

use unicode_width::UnicodeWidthStr;

use super::bar::clip;
use super::commands::SlashCmd;

// L09: the slash suggestion popup shows at most this many rows.
pub const SLASH_MAX_ROWS: usize = 8;

// Rows of the slash menu for `items`, starting at a window that keeps `sel` visible. `max` caps the row
// count (the caller passes what fits). Each row is clipped to `width` columns.
pub fn slash_popup(items: &[&SlashCmd], sel: usize, width: usize, max: usize) -> Vec<String> {
    let n = items.len().min(max).min(SLASH_MAX_ROWS);
    if n == 0 {
        return Vec::new();
    }
    let sel = sel.min(items.len() - 1);
    let start = if sel >= n { sel + 1 - n } else { 0 };
    let name_w = items
        .iter()
        .map(|c| UnicodeWidthStr::width(c.name))
        .max()
        .unwrap_or(0);
    items[start..start + n]
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let mark = if start + i == sel { "> " } else { "  " };
            let pad = " ".repeat(name_w - UnicodeWidthStr::width(c.name) + 2);
            clip(&format!("{mark}{}{pad}{}", c.name, c.desc), width)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::commands::{SLASH_CMDS, suggestions};
    use super::*;

    #[test]
    fn popup_lists_matches_and_marks_the_selected_row() {
        let sg = suggestions("/mo", false, false);
        let rows = slash_popup(&sg, 1, 80, 8);
        assert_eq!(rows.len(), sg.len());
        assert!(rows[0].starts_with("  /model"));
        assert!(rows[1].starts_with("> /mode"));
        assert!(rows[1].contains(sg[1].desc));
    }

    #[test]
    fn popup_window_follows_the_selection_and_respects_the_cap() {
        let all: Vec<&SlashCmd> = SLASH_CMDS.iter().collect();
        let rows = slash_popup(&all, all.len() - 1, 80, 5);
        assert_eq!(rows.len(), 5);
        assert!(rows[4].starts_with("> /quit"));
        assert!(slash_popup(&all, 0, 80, 0).is_empty());
        assert_eq!(slash_popup(&all, 0, 80, 100).len(), SLASH_MAX_ROWS);
    }

    #[test]
    fn popup_rows_never_exceed_the_width() {
        let all: Vec<&SlashCmd> = SLASH_CMDS.iter().collect();
        for r in slash_popup(&all, 0, 12, 8) {
            assert!(UnicodeWidthStr::width(r.as_str()) <= 12);
        }
    }
}
