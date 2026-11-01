mod common;

use pixban_core::{font, CaseMode};

#[test]
fn parses_header_and_glyphs() {
    let f = common::test_font();
    assert_eq!(f.name(), "Test");
    assert_eq!(f.height(), 7);
    assert_eq!(f.baseline(), 7);
    assert_eq!(f.spacing(), 2);
    assert_eq!(f.case(), CaseMode::Upper);
    assert_eq!(f.default_scale(), 8);
    assert_eq!(f.kerning('A', 'I'), -1);
    assert_eq!(f.kerning('I', 'A'), 0);
    assert_eq!(f.glyph('A').unwrap().width(), 6);
    assert_eq!(f.glyph(' ').unwrap().width(), 3);
    assert!(f.glyph('A').unwrap().get(1, 0));
    assert!(!f.glyph('A').unwrap().get(0, 0));
    assert_eq!(f.chars(), vec![' ', 'A', 'I', 'Д']);
}

#[test]
fn missing_fallback_becomes_a_box() {
    let f = common::test_font();
    let fb = f.fallback();
    assert_eq!((fb.width(), fb.height()), (5, 7));
    assert!(fb.get(0, 0) && fb.get(4, 6) && fb.get(0, 3));
    assert!(!fb.get(2, 3));
}

#[test]
fn explicit_fallback_is_used() {
    let src =
        "name: T\nheight: 2\nbaseline: 2\nspacing: 1\ncase: mixed\n---\nchar: fallback\n#.#\n.#.\n";
    let f = font::parse(src).unwrap();
    assert_eq!(f.fallback().width(), 3);
    assert!(f.fallback().get(2, 0));
    assert!(!f.fallback().get(1, 0));
    assert!(f.glyph('x').is_none());
}

#[test]
fn glyph_names_cover_space_and_code_points() {
    let src = "name: T\nheight: 1\nbaseline: 1\nspacing: 0\ncase: mixed\n---\nchar: U+0041\n#\nchar: space\n.\nchar: #\n#\n";
    let f = font::parse(src).unwrap();
    assert_eq!(f.chars(), vec![' ', '#', 'A']);
}

fn header(extra: &str) -> String {
    format!("name: T\nheight: 2\nbaseline: 2\nspacing: 1\ncase: upper\n{extra}---\n")
}

fn fails_at(src: &str, line: usize, needle: &str) {
    let e = font::parse(src).unwrap_err();
    assert_eq!(e.line, line, "wrong line for {src:?}: {e}");
    assert!(
        e.message.contains(needle),
        "message {:?} does not contain {needle:?}",
        e.message
    );
    assert!(e.to_string().starts_with(&format!("line {line}:")));
}

#[test]
fn error_missing_separator_points_past_the_last_line() {
    fails_at("name: T\nheight: 2\n", 3, "---");
}

#[test]
fn error_unknown_header() {
    fails_at("name: T\ncolor: red\n---\n", 2, "unknown header");
}

#[test]
fn error_header_without_colon() {
    fails_at("name: T\nheight 2\n---\n", 2, "key: value");
}

#[test]
fn error_bad_number() {
    fails_at("name: T\nheight: tall\n---\n", 2, "needs a number");
}

#[test]
fn error_duplicate_header() {
    fails_at("name: T\nname: U\n---\n", 2, "duplicate");
}

#[test]
fn error_missing_required_header_points_at_separator() {
    fails_at(
        "name: T\nheight: 2\nbaseline: 2\ncase: upper\n---\n",
        5,
        "spacing",
    );
}

#[test]
fn error_baseline_beyond_height() {
    fails_at(
        "name: T\nheight: 2\nbaseline: 3\nspacing: 1\ncase: upper\n---\n",
        3,
        "baseline",
    );
}

#[test]
fn error_bad_case() {
    fails_at(
        "name: T\nheight: 2\nbaseline: 2\nspacing: 1\ncase: title\n---\n",
        5,
        "upper",
    );
}

#[test]
fn error_bad_kern() {
    fails_at(&header("kern: ABC 1\n"), 6, "two characters");
    fails_at(&header("kern: AB x\n"), 6, "not a number");
    fails_at(&header("kern: AB 99\n"), 6, "between");
    fails_at(&header("kern: AB 1\nkern: AB 2\n"), 7, "duplicate");
}

#[test]
fn error_too_few_rows() {
    let src = header("") + "char: A\n##\n\nchar: B\n##\n##\n";
    fails_at(&src, 9, "has 1 rows, expected 2");
}

#[test]
fn error_too_few_rows_at_end_of_file() {
    let src = header("") + "char: A\n##\n";
    fails_at(&src, 9, "expected 2");
}

#[test]
fn error_too_many_rows() {
    let src = header("") + "char: A\n##\n##\n##\n";
    fails_at(&src, 10, "more than 2 rows");
}

#[test]
fn error_invalid_row_character() {
    let src = header("") + "char: A\n##\n#x\n";
    fails_at(&src, 9, "invalid character 'x'");
}

#[test]
fn error_uneven_row_width() {
    let src = header("") + "char: A\n###\n##\n";
    fails_at(&src, 9, "wide");
}

#[test]
fn error_duplicate_glyph() {
    let src = header("") + "char: A\n##\n##\nchar: A\n##\n##\n";
    fails_at(&src, 10, "duplicate glyph");
}

#[test]
fn error_bad_glyph_name() {
    let src = header("") + "char: AB\n##\n##\n";
    fails_at(&src, 7, "invalid glyph name");
}

#[test]
fn error_text_where_glyph_expected() {
    let src = header("") + "hello\n";
    fails_at(&src, 7, "char:");
}

#[test]
fn crlf_and_bom_are_accepted() {
    let src = format!("\u{feff}{}", common::SRC.replace('\n', "\r\n"));
    assert_eq!(font::parse(&src).unwrap().chars().len(), 4);
}
