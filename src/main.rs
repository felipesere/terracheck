use tree_sitter::{Parser, Language, Query, QueryCursor, QueryMatch};
use std::path::PathBuf;

fn main() {
    extern "C" { fn tree_sitter_terraform() -> Language; }

    let mut parser = Parser::new();
    let language = unsafe { tree_sitter_terraform() };
    parser.set_language(language).unwrap();

    let path_to_file: PathBuf = ["sample.tf"].iter().collect();
    let content = std::fs::read_to_string(&path_to_file).unwrap();

    let tree = parser.parse(&content, None).unwrap();

    let query = Query::new(language, "(attribute (identifier) @i (number) @v)").expect("unworkable query");

    let root = tree.root_node();

    println!("{}", root.to_sexp());

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, root, |node| {
        "hi"
    });

    for m in matches {
        for capture in m.captures {
            println!("{}", capture.node.utf8_text(&content.as_bytes()).unwrap());
        }
    }
}
