//! Display width of one char, go-runewidth v0.0.16 RuneWidth. Terminal-free, so the TUI and the GUI editor share it.

use crate::runewidth_table::{AMBIGUOUS, COMBINING, DOUBLEWIDTH, NARROW, NONPRINT};

// RuneWidth for one rune. Zero-width marks are 0 as in unicode-width. Go gives the Hangul jamo block
// U+1160..U+11FF width 1 (unicode-width says 0), and U+FE0F width 2 under EastAsianWidth, else 1.
pub fn char_width_ea(c: char, ea: bool) -> usize {
    let r = c as u32;
    match r {
        0x1160..=0x11FF => return 1,
        0xFE0F => return if ea { 2 } else { 1 },
        _ => {}
    }
    // go-runewidth v0.0.16 RuneWidth with StrictEmojiNeutral=true (the default).
    if !ea {
        return if r < 0x20 || (0x7F..=0x9F).contains(&r) || r == 0xAD {
            0
        } else if r < 0x300 || in_table(r, NARROW) {
            1
        } else if in_table(r, NONPRINT) || in_table(r, COMBINING) {
            0
        } else if in_table(r, DOUBLEWIDTH) {
            2
        } else {
            1
        };
    }
    if in_table(r, NONPRINT) || in_table(r, COMBINING) {
        0
    } else if in_table(r, NARROW) {
        1
    } else if in_table(r, AMBIGUOUS) || in_table(r, DOUBLEWIDTH) {
        2
    } else {
        1
    }
}

// Binary search over sorted inclusive (first, last) ranges (go-runewidth inTable).
fn in_table(r: u32, t: &[(u32, u32)]) -> bool {
    t.binary_search_by(|&(lo, hi)| {
        if hi < r {
            std::cmp::Ordering::Less
        } else if lo > r {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    })
    .is_ok()
}
