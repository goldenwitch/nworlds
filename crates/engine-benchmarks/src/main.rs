#![forbid(unsafe_code)]

use std::{env, fs, path::PathBuf, process};

use engine_benchmarks::{
    render_json, run, BenchmarkConfig, DEFAULT_ITERATIONS, DEFAULT_WARMUP_ITERATIONS,
};

fn main() {
    let (config, report_path) = match arguments() {
        Ok(values) => values,
        Err(message) => {
            eprintln!("{message}");
            process::exit(2);
        }
    };

    let report = run(config);
    let command = format!(
        "cargo run --release --manifest-path crates/engine-benchmarks/Cargo.toml -- --iterations {} --warmup {} --report {}",
        config.iterations,
        config.warmup_iterations,
        report_path.display(),
    );
    let json = render_json(&report, &command);

    if let Some(parent) = report_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        if let Err(error) = fs::create_dir_all(parent) {
            eprintln!(
                "cannot create report directory {}: {error}",
                parent.display()
            );
            process::exit(1);
        }
    }

    if let Err(error) = fs::write(&report_path, json) {
        eprintln!(
            "cannot write benchmark report {}: {error}",
            report_path.display()
        );
        process::exit(1);
    }

    println!(
        "benchmark report: {} (profile={}, iterations={}, warmup={})",
        report_path.display(),
        report.profile,
        config.iterations,
        config.warmup_iterations,
    );
}

fn arguments() -> Result<(BenchmarkConfig, PathBuf), String> {
    let mut iterations = DEFAULT_ITERATIONS;
    let mut warmup_iterations = DEFAULT_WARMUP_ITERATIONS;
    let mut report_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../evidence/benchmarks/anchor-report.json");
    let mut arguments = env::args().skip(1);

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--iterations" => {
                iterations = parse_positive(
                    "--iterations",
                    arguments.next().ok_or("--iterations needs a value")?,
                )?;
            }
            "--warmup" => {
                warmup_iterations = parse_positive(
                    "--warmup",
                    arguments.next().ok_or("--warmup needs a value")?,
                )?;
            }
            "--report" => {
                report_path = PathBuf::from(arguments.next().ok_or("--report needs a path")?);
            }
            "--help" | "-h" => {
                return Err(usage());
            }
            unknown => return Err(format!("unknown argument {unknown}\n\n{}", usage())),
        }
    }

    Ok((
        BenchmarkConfig {
            iterations,
            warmup_iterations,
        },
        report_path,
    ))
}

fn parse_positive(name: &str, value: String) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("{name} must be a positive integer, got {value}"))?;
    if parsed == 0 {
        return Err(format!("{name} must be a positive integer"));
    }
    Ok(parsed)
}

fn usage() -> String {
    "usage: engine-benchmarks [--iterations N] [--warmup N] [--report PATH]".to_owned()
}
