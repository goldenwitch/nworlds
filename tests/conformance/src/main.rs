use std::{
    env, fs,
    panic::{catch_unwind, AssertUnwindSafe},
    path::{Path, PathBuf},
    process,
};

use caravan_conformance::{cases, GAPS};

fn main() {
    let output = report_path();
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).expect("create evidence directory");
    }

    let mut rows = String::new();
    let mut failed = false;
    for case in cases() {
        let result = catch_unwind(AssertUnwindSafe(|| (case.run)()));
        let status = if result.is_ok() {
            "pass"
        } else {
            failed = true;
            "fail"
        };
        if !rows.is_empty() {
            rows.push_str(",\n");
        }
        rows.push_str(&format_case(case, status));
    }

    let gaps = GAPS
        .iter()
        .map(|gap| {
            format!(
                "    {{\"id\":\"{}\",\"status\":\"{}\",\"description\":\"{}\"}}",
                escape(gap.id),
                escape(gap.status),
                escape(gap.description)
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let report = format!(
        "{{\n  \"schema\": \"caravan-conformance-report-v1\",\n  \"status\": \"{}\",\n  \"command\": \"cargo test --manifest-path tests/conformance/Cargo.toml\",\n  \"cases\": [\n{}\n  ],\n  \"gaps\": [\n{}\n  ]\n}}\n",
        if failed { "fail" } else { "pass" },
        rows,
        gaps
    );
    fs::write(&output, report).expect("write conformance report");
    println!("conformance report: {}", output.display());

    if failed {
        process::exit(1);
    }
}

fn report_path() -> PathBuf {
    let mut args = env::args_os().skip(1);
    while let Some(argument) = args.next() {
        if argument == "--report" {
            if let Some(path) = args.next() {
                return PathBuf::from(path);
            }
        }
    }

    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../evidence/conformance-report.json")
}

fn format_case(case: &caravan_conformance::Case, status: &str) -> String {
    format!(
        "    {{\"id\":\"{}\",\"status\":\"{}\",\"clause\":\"{}\",\"test\":\"{}\",\"artifact\":\"{}\"}}",
        escape(case.id),
        escape(status),
        escape(case.clause),
        escape(case.test),
        escape(case.artifact)
    )
}

fn escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
