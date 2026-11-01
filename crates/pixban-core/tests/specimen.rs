mod common;

use pixban_core::{specimen, Error};

#[test]
fn scale_must_be_an_integer_in_range() {
    let f = common::test_font();
    assert_eq!(
        specimen(&f, 0).unwrap_err(),
        Error::InvalidScale { scale: 0, max: 64 }
    );
    assert!(matches!(
        specimen(&f, 65),
        Err(Error::InvalidScale { scale: 65, .. })
    ));
}

#[test]
fn sheet_is_wider_than_its_longest_line_and_scales_exactly() {
    let f = common::test_font();
    let one = specimen(&f, 1).unwrap();
    let two = specimen(&f, 2).unwrap();
    assert_eq!(one.grid.width(), two.grid.width());
    assert_eq!(
        two.pixel_size(),
        (one.grid.width() * 2, one.grid.height() * 2)
    );
    assert!(one.grid.width() >= 40 + 6);
    assert!(!one.to_png().unwrap().is_empty());
}
