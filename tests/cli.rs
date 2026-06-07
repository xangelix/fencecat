use std::{process::Command, str};

#[test]
fn test_hidden_flag_behavior() {
    // 1. Run with `--hidden` to verify hidden files (like those in `.github/`) are included.
    let output_long = Command::new("cargo")
        .args(["run", "--bin", "fencecat", "--", ".", "--hidden"])
        .output()
        .expect("failed to execute cargo run");

    assert!(output_long.status.success());
    let stdout_long = str::from_utf8(&output_long.stdout).expect("invalid utf8 output");

    // Verify that .github workflow files are included as walked paths (fence header lines)
    let has_hidden_path_long = stdout_long.lines().any(|line| {
        line.starts_with('`')
            && (line.contains(".github/workflows/ci.yml")
                || line.contains(".github/workflows/release.yml"))
    });
    assert!(
        has_hidden_path_long,
        "Expected .github files to be present when --hidden is active"
    );

    // Verify that gitignored paths (such as files in 'target/') are not included as walked paths.
    let contains_target_path = stdout_long
        .lines()
        .any(|line| line.starts_with('`') && line.contains("target/"));
    assert!(
        !contains_target_path,
        "Expected gitignored directories (like target/) to remain excluded"
    );

    // 2. Run with short flag `-.` to verify parsing.
    let output_short = Command::new("cargo")
        .args(["run", "--bin", "fencecat", "--", ".", "-."])
        .output()
        .expect("failed to execute cargo run");

    assert!(output_short.status.success());
    let stdout_short = str::from_utf8(&output_short.stdout).expect("invalid utf8 output");

    let has_hidden_path_short = stdout_short.lines().any(|line| {
        line.starts_with('`')
            && (line.contains(".github/workflows/ci.yml")
                || line.contains(".github/workflows/release.yml"))
    });
    assert!(
        has_hidden_path_short,
        "Expected .github files to be present when short flag -. is active"
    );

    // 3. Run without the hidden flag to verify hidden files are excluded by default.
    let output_default = Command::new("cargo")
        .args(["run", "--bin", "fencecat", "--", "."])
        .output()
        .expect("failed to execute cargo run");

    assert!(output_default.status.success());
    let stdout_default = str::from_utf8(&output_default.stdout).expect("invalid utf8 output");

    // Verify that no hidden directory file paths are included in the walked paths
    let contains_hidden_path_default = stdout_default
        .lines()
        .any(|line| line.starts_with('`') && line.contains(".github/"));
    assert!(
        !contains_hidden_path_default,
        "Expected hidden files to be excluded by default"
    );
}
