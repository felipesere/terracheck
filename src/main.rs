use glob::glob;
use std::env;
use std::fs::read_to_string;
use tree_sitter::{Language, Parser, Query, QueryCursor, Node};

fn main() {
    extern "C" {
        fn tree_sitter_terraform() -> Language;
    }

    let mut parser = Parser::new();
    let language = unsafe { tree_sitter_terraform() };
    parser.set_language(language).unwrap();

    let file = env::args().skip(1).next().expect("Need a path to a query");
    let content = read_to_string(file).unwrap();

    let query = Query::new(language, &content).expect("unworkable query");

    let mut cursor = QueryCursor::new();
    for entry in glob("**/*.tf").expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let content = read_to_string(&path).unwrap();

                let tree = parser.parse(&content, None).unwrap();

                let text_callback = |n: Node| &content[n.byte_range()];

                let mut matches = cursor
                    .matches(&query, tree.root_node(), text_callback)
                    .peekable();

                if matches.peek().is_none() {
                    continue;
                }

                println!("\n{}", path.to_str().unwrap());

                for m in matches {
                    for capture in m.captures {
                        let name = &query.capture_names()[capture.index as usize];

                        if name != "result" {
                            continue;
                        }
                        println!("{}", capture.node.utf8_text(&content.as_bytes()).unwrap());
                        println!("{}", capture.node.to_sexp());
                        println!("");
                    }
                }
            }
            Err(e) => println!("{:?}", e),
        }
    }
}
