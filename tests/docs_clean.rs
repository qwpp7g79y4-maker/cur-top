//! The public documents must not point at the machine, the paths, or the
//! tools that built them. The patterns are general (absolute local paths,
//! option prefixes of local wrappers, names of local work files). Names that
//! must not appear in the documents are built from pieces, so this source
//! does not hold them either.

use std::fs;
use std::path::{Path, PathBuf};

fn forbidden() -> Vec<String> {
    [
        // Absolute local paths.
        "/media/",
        "/home/",
        "~/",
        // Option prefixes of local wrappers.
        concat!("--max", "-gb"),
        concat!("--na", "am"),
        // Local work files.
        concat!("tas", "k.md"),
        concat!("blue", "print.md"),
        // Names of local tools and folders.
        concat!("geheugen", "wachter"),
        concat!("bouw", "-tmp"),
        concat!("brug", "-prompts"),
        // Names of AI systems and bots.
        concat!("open", "bot"),
        concat!("son", "net"),
        concat!("gr", "ok"),
        concat!("cla", "ude"),
        concat!("anthr", "opic"),
    ]
    .iter()
    .map(|pattern| pattern.to_string())
    .collect()
}

/// The patterns found in `text`, compared without regard to case.
fn found_in(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    forbidden()
        .into_iter()
        .filter(|pattern| lower.contains(pattern.as_str()))
        .collect()
}

fn documents() -> Vec<(String, String)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files: Vec<PathBuf> = [
        "README.md",
        "README.nl.md",
        "QUALITY.md",
        "QUALITY.nl.md",
        "CHANGELOG.md",
        "SECURITY.md",
        "CONTRIBUTING.md",
        "THIRD-PARTY-LICENSES.html",
    ]
    .iter()
    .map(|name| root.join(name))
    .collect();
    for entry in fs::read_dir(root.join("man")).unwrap() {
        files.push(entry.unwrap().path());
    }
    files
        .iter()
        .map(|path| (label(&root, path), fs::read_to_string(path).unwrap()))
        .collect()
}

fn label(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap().display().to_string()
}

#[test]
fn public_documents_hold_no_internal_references() {
    let docs = documents();
    assert!(docs.len() >= 8, "expected seven documents and a man page");
    for (name, text) in &docs {
        assert!(!text.is_empty(), "{name} is empty");
        let hits = found_in(text);
        assert!(hits.is_empty(), "{name} contains {hits:?}");
    }
}

/// The source, the workflows, and the other repository files must not hold
/// the patterns either. `.gitignore` and `Cargo.toml` name a local work file
/// on purpose, so they are not read.
fn other_files() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else {
                out.push(path);
            }
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // `.gitattributes` is excluded from the package, so it only exists in the repository.
    let mut files: Vec<PathBuf> = [root.join("deny.toml"), root.join(".gitattributes")]
        .into_iter()
        .filter(|path| path.is_file())
        .collect();
    for dir in ["src", ".github", "examples", "scripts"] {
        if root.join(dir).is_dir() {
            walk(&root.join(dir), &mut files);
        }
    }
    files
        .iter()
        .map(|path| (label(&root, path), fs::read_to_string(path).unwrap()))
        .collect()
}

#[test]
fn source_workflows_and_scripts_hold_no_internal_references() {
    let files = other_files();
    assert!(files.len() >= 10, "expected the sources and the workflow");
    for (name, text) in &files {
        let hits = found_in(text);
        assert!(hits.is_empty(), "{name} contains {hits:?}");
    }
}

#[test]
fn the_patterns_catch_each_kind_of_reference() {
    for pattern in forbidden() {
        let upper = pattern.to_uppercase();
        for bad in [format!("see {pattern} here"), format!("see {upper} here")] {
            assert!(!found_in(&bad).is_empty(), "not caught: {bad}");
        }
    }
    assert!(found_in("cur-top examples/cur.csv --top 3").is_empty());
}

#[test]
fn about_toml_accepts_the_same_licenses_as_deny_toml() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let list = |file: &str, key: &str| -> Vec<String> {
        let text = std::fs::read_to_string(root.join(file)).unwrap();
        let start = text.find(&format!("{key} = [")).unwrap();
        let body = &text[start..];
        let body = &body[body.find('[').unwrap() + 1..body.find(']').unwrap()];
        let mut out: Vec<String> = body
            .split(',')
            .map(|item| item.trim().trim_matches('"').to_string())
            .filter(|item| !item.is_empty())
            .collect();
        out.sort();
        out
    };
    assert_eq!(list("about.toml", "accepted"), list("deny.toml", "allow"));
    let template = std::fs::read_to_string(root.join("about.hbs")).unwrap();
    assert!(template.contains("Third-party licenses"));
}
