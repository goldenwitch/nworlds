use std::process::Command;

#[test]
fn terminal_trace_matches_snapshot() {
    let output = Command::new(env!("CARGO_BIN_EXE_caravan-demo"))
        .output()
        .expect("cargo provides the demo binary for integration tests");

    assert!(
        output.status.success(),
        "demo exited unsuccessfully: {:?}",
        output.status
    );
    assert!(
        output.stderr.is_empty(),
        "demo wrote to stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("demo output is UTF-8");
    let expected = include_str!("../snapshots/anchor-trace.txt").replace("\r\n", "\n");
    assert_eq!(stdout.replace("\r\n", "\n"), expected);
}
