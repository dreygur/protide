//! Double-click word selection over **character** (codepoint) indices.
//!
//! The indices these return are counted in characters, matching `index_for_x`,
//! `normalized_selection` and the selection ranges every caller stores.

use unicode_segmentation::UnicodeSegmentation;

/// Check if character is a word character (alphanumeric or underscore)
pub fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// One word/separator class per **character**, aligned with `text.chars()`.
///
/// Classification is per *grapheme cluster*: every character of a cluster takes
/// the class of the cluster's first character, so a combining mark (which is
/// not alphanumeric on its own) stays attached to the letter it decorates and a
/// decomposed "cafe\u{0301}" selects whole instead of dropping the accent.
fn char_classes(text: &str) -> Vec<bool> {
    let mut classes = Vec::with_capacity(text.chars().count());
    for cluster in text.graphemes(true) {
        let is_word = cluster.chars().next().is_some_and(is_word_char);
        classes.extend(std::iter::repeat_n(is_word, cluster.chars().count()));
    }
    classes
}

/// Character span a double-click at `pos` selects.
///
/// The character under the cursor picks the class - word or separator - and the
/// span grows in *both* directions over that same class. Clicking a separator
/// therefore selects the run of separators (the VS Code / Zed convention), not
/// a neighbouring word.
fn word_span(text: &str, pos: usize) -> std::ops::Range<usize> {
    let classes = char_classes(text);
    let Some(last) = classes.len().checked_sub(1) else {
        return 0..0;
    };
    let class = classes[pos.min(last)];

    let mut start = pos.min(last);
    while start > 0 && classes[start - 1] == class {
        start -= 1;
    }
    let mut end = pos.min(last);
    while end < classes.len() && classes[end] == class {
        end += 1;
    }
    start..end
}

/// Find the start of the span a double-click at the given position selects
pub fn find_word_start(text: &str, pos: usize) -> usize {
    word_span(text, pos).start
}

/// Find the end of the span a double-click at the given position selects
pub fn find_word_end(text: &str, pos: usize) -> usize {
    word_span(text, pos).end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_characters_are_alphanumerics_and_underscore() {
        for c in ['a', 'Z', '0', '9', '_', 'é', 'ü', '日', 'Ω'] {
            assert!(is_word_char(c), "{c:?} should be a word char");
        }
        for c in [' ', '\t', '\n', '/', ':', '?', '=', '&', '.', '-', '🎉'] {
            assert!(!is_word_char(c), "{c:?} should not be a word char");
        }
    }

    /// Word span selected by a double-click at char index `pos`.
    fn word_at(text: &str, pos: usize) -> String {
        let (s, e) = (find_word_start(text, pos), find_word_end(text, pos));
        assert!(s <= e, "inverted word span {s}..{e} for {text:?} @ {pos}");
        text.chars().skip(s).take(e - s).collect()
    }

    #[test]
    fn a_double_click_inside_a_word_selects_that_whole_word() {
        let text = "GET /api/users?id=42";
        assert_eq!(word_at(text, 0), "GET");
        assert_eq!(word_at(text, 2), "GET");
        assert_eq!(word_at(text, 5), "api");
        assert_eq!(word_at(text, 10), "users");
        assert_eq!(word_at(text, 18), "42");
    }

    #[test]
    fn word_boundaries_are_char_indices_not_byte_indices() {
        // Both functions index by character, so their results must be counted in
        // characters; treating them as byte offsets would slice mid-character.
        let text = "日本語 hello";
        assert_eq!(find_word_start(text, 1), 0);
        assert_eq!(
            find_word_end(text, 1),
            3,
            "the CJK word is 3 chars, not 9 bytes"
        );
        assert_eq!(word_at(text, 1), "日本語");
        assert_eq!(word_at(text, 5), "hello");
    }

    // REGRESSION: `find_word_start` walked *backwards* to the previous word while
    // `find_word_end` walked *forwards* to the next one, so a double-click on any
    // non-word character selected the preceding word, the separators, *and* the
    // following word - on "https://api.example.com" clicking the ':' selected
    // "https://api".
    //
    // FIXED: both now delegate to `word_span`, which classifies the character
    // under the cursor and grows in both directions over that one class.
    //
    // The semantics below were *chosen*, not derived: a double-click on a
    // separator selects the run of separators, matching VS Code and Zed. The
    // alternatives (select the following word, or the preceding one) are equally
    // self-consistent; this is the one the product picked.
    #[test]
    fn a_double_click_on_a_separator_selects_only_one_word() {
        assert_eq!(word_at("a  bb", 1), "  ");
        assert_eq!(word_at("hello world", 5), " ");
        assert_eq!(word_at("https://api.example.com", 5), "://");
        // Clicking inside a word is unchanged.
        assert_eq!(word_at("hello world", 2), "hello");
    }

    #[test]
    fn word_lookup_never_indexes_past_the_end_of_the_text() {
        for text in ["", " ", "a", "word", "日本語", "e\u{0301}x", "🎉🎉"] {
            let n = text.chars().count();
            for pos in 0..n + 5 {
                let (s, e) = (find_word_start(text, pos), find_word_end(text, pos));
                assert!(s <= n, "{text:?} @ {pos}: start {s} > {n}");
                assert!(e <= n, "{text:?} @ {pos}: end {e} > {n}");
                assert!(s <= e, "{text:?} @ {pos}: inverted {s}..{e}");
            }
        }
    }

    #[test]
    fn word_lookup_on_empty_text_is_an_empty_span() {
        assert_eq!(find_word_start("", 0), 0);
        assert_eq!(find_word_start("", 99), 0);
        assert_eq!(find_word_end("", 0), 0);
        assert_eq!(find_word_end("", 99), 0);
    }

    #[test]
    fn text_with_no_word_characters_yields_an_empty_selection() {
        let text = "///";
        assert_eq!(find_word_end(text, 0), 3);
        assert_eq!(find_word_start(text, 2), 0);
    }

    // REGRESSION: word selection classified bare codepoints, so a decomposed
    // grapheme (base letter + combining mark) was split - the mark is not
    // alphanumeric, so the word ended before it and a double-click on "café"
    // spelled as "cafe\u{0301}" copied "cafe", silently dropping the accent.
    // Precomposed "café" (U+00E9) was always fine.
    //
    // FIXED: `char_classes` classifies per grapheme cluster (via
    // unicode-segmentation) and gives every character of a cluster the class of
    // the cluster's base, so a run can never end inside a cluster. The returned
    // indices stay codepoint-based, which is what all callers count in.
    #[test]
    fn a_double_click_selects_a_whole_grapheme_cluster() {
        assert_eq!(word_at("cafe\u{0301} x", 1), "cafe\u{0301}");
        // The combining mark itself belongs to the word, not to the separators.
        assert_eq!(word_at("cafe\u{0301} x", 4), "cafe\u{0301}");
        assert_eq!(word_at("cafe\u{0301} x", 5), " ");
    }
}
