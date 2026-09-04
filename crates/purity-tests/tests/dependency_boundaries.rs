use std::{fs, path::PathBuf};

const LIBRARY_MANIFESTS: &[&str] = &[
    "crates/engine-time/Cargo.toml",
    "crates/engine-sdk/Cargo.toml",
    "crates/engine-journal/Cargo.toml",
    "crates/engine-branches/Cargo.toml",
    "crates/engine-index/Cargo.toml",
    "crates/engine-presentation/Cargo.toml",
    "crates/engine-api/Cargo.toml",
];

const FORBIDDEN_PRODUCTION_DEPENDENCIES: &[&str] = &[
    "caravan",
    "caravan-demo",
    "nworlds-host",
    "nworlds-desktop",
    "winit",
    "wgpu",
];

#[test]
fn library_production_manifests_do_not_depend_on_consumers() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut violations = Vec::new();

    for relative_manifest in LIBRARY_MANIFESTS {
        let path = workspace_root.join(relative_manifest);
        let manifest = fs::read_to_string(&path).expect("library manifest should be readable");
        let mut section = String::new();

        for line in manifest.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                section = trimmed.to_ascii_lowercase();
                continue;
            }
            if !is_production_dependency_section(&section) {
                continue;
            }
            let Some((name, _)) = trimmed.split_once('=') else {
                continue;
            };
            let name = name.trim().trim_matches('"').to_ascii_lowercase();
            if FORBIDDEN_PRODUCTION_DEPENDENCIES
                .iter()
                .any(|forbidden| name.contains(forbidden))
            {
                violations.push(format!("{relative_manifest}: {name}"));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "library production dependency violations:\n{}",
        violations.join("\n")
    );
}

fn is_production_dependency_section(section: &str) -> bool {
    section == "[dependencies]"
        || (section.ends_with(".dependencies]") && !section.contains("dev-dependencies"))
}
