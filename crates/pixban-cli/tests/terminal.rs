use std::process::{Command, Output};

fn run(args: &[&str], env: &[(&str, &str)]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pixban"))
        .args(args)
        .env_clear()
        .envs(env.iter().copied())
        .output()
        .unwrap()
}

fn ok(args: &[&str], env: &[(&str, &str)]) -> String {
    let out = run(args, env);
    assert!(
        out.status.success(),
        "pixban {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

const ANCHOR: [&str; 7] = ["brazier", "-f", "block", "-p", "fire", "--shadow", "1,1"];

fn anchor(extra: &[&str]) -> String {
    let mut args = ANCHOR.to_vec();
    args.extend_from_slice(extra);
    ok(&args, &[])
}

#[test]
fn golden_truecolor() {
    insta::assert_snapshot!(format!(
        "{:?}",
        anchor(&["--ansi", "--color-mode", "truecolor"])
    ));
}

#[test]
fn golden_256() {
    insta::assert_snapshot!(format!("{:?}", anchor(&["--ansi", "--color-mode", "256"])));
}

#[test]
fn golden_16() {
    insta::assert_snapshot!(format!("{:?}", anchor(&["--ansi", "--color-mode", "16"])));
}

#[test]
fn golden_none() {
    insta::assert_snapshot!(anchor(&["--ansi", "--color-mode", "none"]));
}

#[test]
fn golden_ascii() {
    insta::assert_snapshot!(anchor(&[
        "--ansi",
        "--color-mode",
        "none",
        "--blocks",
        "ascii"
    ]));
}

#[test]
fn piped_output_stays_svg_without_escapes() {
    let out = anchor(&[]);
    assert!(out.starts_with("<svg"));
    assert!(!out.contains('\x1b'));
}

#[test]
fn forced_ansi_detects_the_terminal_from_the_environment() {
    let go = |env: &[(&str, &str)]| {
        let mut args = ANCHOR.to_vec();
        args.push("--ansi");
        ok(&args, env)
    };
    assert!(go(&[("COLORTERM", "truecolor"), ("TERM", "xterm")]).contains("38;2;"));
    assert!(go(&[("COLORTERM", "24bit")]).contains("38;2;"));
    assert!(go(&[("TERM", "xterm-256color")]).contains("38;5;"));
    let basic = go(&[("TERM", "xterm")]);
    assert!(basic.contains('\x1b') && !basic.contains("38;"));
    assert!(!go(&[]).contains('\x1b'));
    assert!(!go(&[("COLORTERM", "truecolor"), ("NO_COLOR", "1")]).contains('\x1b'));
}

#[test]
fn explicit_color_mode_beats_the_environment() {
    let mut args = ANCHOR.to_vec();
    args.extend(["--ansi", "--color-mode", "truecolor"]);
    assert!(ok(&args, &[("NO_COLOR", "1")]).contains("38;2;"));
}

#[test]
fn a_logo_wider_than_the_terminal_is_refused() {
    let mut args = ANCHOR.to_vec();
    args.push("--ansi");
    let out = run(&args, &[("COLUMNS", "10")]);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("columns"));
    args.extend(["--blocks", "half"]);
    assert!(run(&args, &[("COLUMNS", "200")]).status.success());
}

#[test]
fn odd_text_does_not_panic() {
    for text in ["e\u{301}", "\u{4e2d}\u{6587}", "Привет", "a\u{200b}b"] {
        let out = run(&[text, "--ansi"], &[]);
        assert!(out.status.code().is_some_and(|c| c <= 1), "{text}");
    }
}

#[test]
fn animation_prints_the_first_frame_only_when_not_a_terminal() {
    let still = ok(&["hi", "--ansi", "--color-mode", "none"], &[]);
    let anim = ok(
        &[
            "hi",
            "--anim",
            "wave",
            "--ansi",
            "--color-mode",
            "truecolor",
        ],
        &[],
    );
    assert!(!anim.contains("\x1b[?25"));
    assert!(!anim.contains("\x1b[1A"));
    assert!(anim.lines().count() >= still.lines().count());
    assert!(anim.lines().count() < 20);
}

const TRUE: [(&str, &str); 1] = [("COLORTERM", "truecolor")];

#[test]
fn snippet_prints_command_and_escaped_text() {
    let shell = ok(&["snippet", "--for", "shell", "hi", "-p", "fire"], &TRUE);
    assert!(shell.contains("# regenerate: pixban hi -p fire --ansi --color-mode truecolor"));
    assert!(shell.contains("printf '%s' $'") && shell.contains("\\x1b[38;2;"));
    assert!(!shell.contains('\x1b'));
    let py = ok(&["snippet", "--for", "python", "hi"], &TRUE);
    assert!(py.contains("print(\"") && py.contains("end=\"\")"));
    let rs = ok(
        &["snippet", "--for", "rust", "hi", "--shadow", "1,1"],
        &TRUE,
    );
    assert!(rs.contains("print!(\"") && rs.contains("\\n"));
    assert!(rs.contains("pixban hi --shadow 1,1 --ansi --color-mode truecolor"));
    let quoted = ok(&["snippet", "two words"], &TRUE);
    assert!(quoted.contains("'two words'"));
}

#[test]
fn snippet_resolves_the_color_mode_and_writes_it_down() {
    type Env<'a> = &'a [(&'a str, &'a str)];
    let cases: [(Env, &str, &str); 5] = [
        (&[("NO_COLOR", "1"), ("COLORTERM", "truecolor")], "none", ""),
        (&TRUE, "truecolor", "38;2;"),
        (&[("TERM", "xterm-256color")], "256", "38;5;"),
        (&[("TERM", "xterm")], "16", "[3"),
        (&[], "none", ""),
    ];
    for (env, mode, mark) in cases {
        let out = ok(&["snippet", "hi"], env);
        assert!(
            out.contains(&format!("--ansi --color-mode {mode}\n")),
            "{mode}: {out}"
        );
        assert!(mark.is_empty() || out.contains(mark), "{mode}");
        if mode == "none" {
            assert!(!out.contains("\\x1b"), "{out}");
        }
        let direct = ok(&["hi", "--ansi", "--color-mode", mode], &[]);
        let body = out
            .split_once("$'")
            .unwrap()
            .1
            .trim_end()
            .trim_end_matches('\'');
        assert_eq!(body.matches("\\n").count(), direct.matches('\n').count());
    }
    let out = ok(&["snippet", "hi", "--color-mode", "16"], &TRUE);
    assert!(out.contains("--ansi --color-mode 16\n") && !out.contains("truecolor"));
    let out = ok(&["snippet", "hi", "--color-mode=256", "--ansi"], &TRUE);
    assert_eq!(out.matches("--color-mode").count(), 1);
}

#[test]
fn ansi_with_an_output_file_is_refused() {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ansi-refused.svg");
    let _ = std::fs::remove_file(&dir);
    let out = run(&["hi", "--ansi", "-o", dir.to_str().unwrap()], &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty() && !dir.exists());
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(err.lines().count(), 1);
    assert!(err.contains("--ansi") && err.contains("redirect"));
}

#[test]
fn snippet_rejects_bad_settings() {
    let out = run(&["snippet", "hi", "--nonsense"], &[]);
    assert!(!out.status.success());
}

#[test]
fn fonts_preview_draws_each_name() {
    let out = ok(&["fonts", "--preview"], &[("COLUMNS", "100")]);
    assert!(out.lines().count() > 8);
    let plain = ok(&["fonts"], &[]);
    assert!(plain.lines().all(|l| l.chars().all(char::is_alphanumeric)));
}

#[test]
fn palettes_preview_draws_swatches() {
    let out = ok(
        &["palettes", "--preview", "--ansi"],
        &[("TERM", "xterm-256color")],
    );
    assert!(out.contains("fire") && out.contains("38;5;"));
    assert!(out
        .lines()
        .all(|l| l.is_empty() || l.ends_with("\x1b[0m") || l.contains("\x1b[0m ")));
}

#[test]
fn specimen_without_output_shows_the_sheet() {
    let out = ok(&["specimen", "-f", "tiny"], &[("COLUMNS", "80")]);
    assert!(out.lines().count() > 5);
    assert!(out.lines().all(|l| l.chars().count() <= 80));
    assert!(!out.contains('\x1b'));
}

fn closed_early(args: &[&str]) -> Output {
    use std::process::Stdio;
    let mut child = Command::new(env!("CARGO_BIN_EXE_pixban"))
        .args(args)
        .env_clear()
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    child.wait_with_output().unwrap()
}

#[test]
fn a_closed_pipe_ends_quietly() {
    let cases: [&[&str]; 4] = [
        &["fonts", "--preview"],
        &["brazier"],
        &["brazier", "--ansi"],
        &["palettes", "--all"],
    ];
    for args in cases {
        let out = closed_early(args);
        assert!(
            out.stderr.is_empty(),
            "{args:?}: {:?}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_ne!(out.status.code(), Some(101), "{args:?}");
    }
}
