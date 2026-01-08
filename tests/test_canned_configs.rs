use morphogenetic_security::config;
use std::fs;
use std::path::PathBuf;

#[test]
fn validate_example_configs() {
    let examples_dir = PathBuf::from("docs/examples");
    if !examples_dir.exists() {
        // Skip if docs missing (e.g. minimal export), but should fail in full repo
        eprintln!("docs/examples not found, skipping validation");
        return;
    }

    let mut found = 0;
    for entry in fs::read_dir(examples_dir).expect("read dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if file_name.starts_with('.') {
            continue;
        }
        if path.extension().map_or(false, |ext| ext == "yaml") {
            println!("Validating {}", path.display());
            config::load_from_path(&path).expect("failed to load config");
            found += 1;
        }
    }
    assert!(found > 0, "No example configs found");
}

#[test]
fn validate_abilene_config() {
    let path = PathBuf::from("data/real_world_samples/abilene_scenario.yaml");
    if !path.exists() {
        eprintln!("Abilene scenario not found, skipping");
        return;
    }
    println!("Validating {}", path.display());
    config::load_from_path(&path).expect("failed to load Abilene config");
}
