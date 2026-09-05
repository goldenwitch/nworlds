use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let revision = git_output(&manifest_dir, &["rev-parse", "--short", "HEAD"])
        .unwrap_or_else(|| "unknown".to_owned());
    let dirty = Command::new("git")
        .args(["-C", &manifest_dir, "diff", "--quiet"])
        .status()
        .map(|status| !status.success())
        .unwrap_or(false);
    let build_id = if dirty {
        format!("{revision}-dirty")
    } else {
        revision
    };

    println!("cargo:rustc-env=NWORLDS_DESKTOP_BUILD_ID={build_id}");
}

fn git_output(manifest_dir: &str, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(["-C", manifest_dir])
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (!value.is_empty()).then_some(value)
}
