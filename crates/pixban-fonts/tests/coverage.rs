use pixban_core::{lint, Script};

const BASELINE: &str = include_str!("coverage_baseline.txt");

#[test]
fn coverage_does_not_drop_below_the_baseline() {
    let mut checked = 0;
    for line in BASELINE
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
    {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let [font, key, min] = parts[..] else {
            panic!("malformed baseline line {line:?}");
        };
        let min: usize = min.parse().expect("baseline count is a number");
        let f = pixban_fonts::get(font).expect("baseline names a built-in font");
        let script = Script::ALL
            .into_iter()
            .find(|s| s.key() == key)
            .expect("baseline names a known script");
        let now = lint(&f)
            .coverage
            .iter()
            .find(|c| c.script == script)
            .expect("every script is reported")
            .covered;
        assert!(now >= min, "{font} {key} covers {now}, baseline is {min}");
        checked += 1;
    }
    assert_eq!(checked, 24, "baseline lists four fonts times six scripts");
}

#[test]
fn builtin_fonts_have_no_lint_errors() {
    for name in pixban_fonts::names() {
        let f = pixban_fonts::get(name).expect("listed font exists");
        let report = lint(&f);
        assert!(report.errors.is_empty(), "{name}: {:?}", report.errors);
    }
}
