use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
struct Children {}

#[derive(Deserialize, Debug)]
struct Node {
    #[serde(rename = "type")]
    kind: String,
    named: bool,
    children: Option<Children>,
}

fn main() {
    let dir: PathBuf = ["..", "..", "tree-sitter-hcl2", "src"].iter().collect();

    let dir_str = dir.to_str().unwrap();

    // Tell Cargo that if the given file changes, to rerun this build script.
    println!("cargo:rerun-if-changed={}", dir_str);

    cc::Build::new()
        .include(&dir)
        .file(dir.join("parser.c"))
        .compile("tree-sitter-hcl2");

    let node_types_json: PathBuf = ["..", "..", "tree-sitter-hcl2", "src", "node-types.json"]
        .iter()
        .collect();

    let f = std::fs::File::open(&node_types_json).unwrap();

    let types: Vec<Node> = serde_json::from_reader(&f).unwrap();

    let containers = types
        .iter()
        .filter(|n| n.children.is_some())
        .map(|n| format!("\"{}\" => true,", n.kind))
        .collect::<Vec<String>>()
        .join("\n");

    let rust_code = format!(
        r#"
        pub fn is_container(kind: &str) -> bool {{
            match kind {{
                {containers}
                _ => false,
            }}
        }}
        "#,
        containers = containers
    );


    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let dest_path = std::path::Path::new(&out_dir).join("is_container.rs");
    std::fs::write(&dest_path, rust_code.as_bytes()).unwrap();
}
