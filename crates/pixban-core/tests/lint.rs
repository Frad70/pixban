use pixban_core::{font, lint, Script};

fn font_from(src: &str) -> pixban_core::Font {
    font::parse(src).unwrap()
}

const CLEAN: &str = "name: L\nheight: 2\nbaseline: 2\nspacing: 1\ncase: mixed\nmax_width: 2\n---\n\
                     char: A\n##\n#.\nchar: a\n..\n##\nchar: space\n..\n..\n\
                     char: fallback\n##\n##\n";

#[test]
fn clean_font_has_no_errors_and_counts_coverage() {
    let report = lint(&font_from(CLEAN));
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let latin_upper = report
        .coverage
        .iter()
        .find(|c| c.script == Script::LatinUpper)
        .unwrap();
    assert_eq!((latin_upper.covered, latin_upper.expected), (1, 26));
    let latin_lower = report
        .coverage
        .iter()
        .find(|c| c.script == Script::LatinLower)
        .unwrap();
    assert_eq!((latin_lower.covered, latin_lower.expected), (1, 26));
}

#[test]
fn caps_fonts_do_not_expect_lowercase() {
    let src = CLEAN.replace("case: mixed", "case: upper");
    let report = lint(&font_from(&src));
    let lower = report
        .coverage
        .iter()
        .find(|c| c.script == Script::LatinLower)
        .unwrap();
    assert_eq!(lower.expected, 0);
}

#[test]
fn expected_sets_match_the_character_list() {
    use pixban_core::{CaseMode, Script as S};
    assert_eq!(S::Punctuation.expected(CaseMode::Upper).len(), 38);
    assert_eq!(S::CyrillicUpper.expected(CaseMode::Upper).len(), 33);
    assert_eq!(S::CyrillicLower.expected(CaseMode::Mixed).len(), 33);
    assert_eq!(S::Digits.expected(CaseMode::Upper).len(), 10);
}

#[test]
fn empty_glyph_and_wide_glyph_are_errors() {
    let src = "name: E\nheight: 1\nbaseline: 1\nspacing: 1\ncase: mixed\nmax_width: 2\n---\n\
               char: A\n.\nchar: B\n###\nchar: fallback\n#\n";
    let report = lint(&font_from(src));
    assert!(report.errors.iter().any(|e| e.contains("'A' has no ink")));
    assert!(report.errors.iter().any(|e| e.contains("'B' is 3 wide")));
}

#[test]
fn space_may_be_empty_but_fallback_may_not() {
    let src = "name: F\nheight: 1\nbaseline: 1\nspacing: 1\ncase: mixed\n---\n\
               char: space\n.\nchar: fallback\n.\n";
    let report = lint(&font_from(src));
    assert_eq!(report.errors.len(), 1, "{:?}", report.errors);
    assert!(report.errors[0].contains("fallback"));
}

#[test]
fn declared_extra_rows_without_ink_are_warned() {
    let src = "name: W\nheight: 1\nbaseline: 1\nspacing: 1\ncase: mixed\n---\n\
               char: A\ntop: 1\n.\n#\nchar: fallback\n#\n";
    let report = lint(&font_from(src));
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert!(report
        .warnings
        .iter()
        .any(|w| w.contains("'A' declares 1 top rows")));
}
