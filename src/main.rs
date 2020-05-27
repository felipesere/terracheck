use glob::glob;
use tree_sitter::{Language, Parser, Query, QueryCursor};
use std::fs::read_to_string;

fn main() {
    extern "C" {
        fn tree_sitter_terraform() -> Language;
    }

    let mut parser = Parser::new();
    let language = unsafe { tree_sitter_terraform() };
    parser.set_language(language).unwrap();

    let query =
        Query::new(language, "(attribute (identifier) @i (number) @v)").expect("unworkable query");

    let mut cursor = QueryCursor::new();
    for entry in glob("**/*.tf").expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let content = read_to_string(&path).unwrap();

                let tree = parser.parse(&content, None).unwrap();

                let matches = cursor.matches(&query, tree.root_node(), |_node| "");

                for m in matches {
                    for capture in m.captures {
                        println!("{}", capture.node.utf8_text(&content.as_bytes()).unwrap());
                    }
                }
            }
            Err(e) => println!("{:?}", e),
        }
    }
}
