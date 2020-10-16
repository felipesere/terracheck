use std::fs::read_to_string;
use std::path::PathBuf;
use std::ops::Range;
use tree_sitter::{Language, Node, Parser, Query, Tree};

extern "C" {
    fn tree_sitter_terraform() -> Language;
}

pub fn query(source: &str) -> Query {
    Query::new(parser().language().unwrap(), source).expect("unworkable query")
}

pub fn parser() -> Parser {
    let mut parser = Parser::new();
    let language = unsafe { tree_sitter_terraform() };
    parser
        .set_language(language)
        .expect("was not able to create the language");

    parser
}

// Surely this can be better than string?
#[derive(Debug, Clone)]
pub struct BackingData {
    tree: Tree,
    pub input: String,
    pub path: String,
}

impl BackingData {
    pub fn text_range(&self, range: &Range<usize>) -> &str {
        &self.input[range.clone()]
    }

    pub fn text(&self, n: Node) -> &str {
        self.text_range(&n.byte_range())
    }

    pub fn root(&self) -> Node {
        self.tree.root_node()
    }
}

pub(crate) fn parse(path: PathBuf) -> BackingData {
    let input = read_to_string(&path).unwrap();
    let mut parser = parser();

    let tree = parser.parse(&input, None).unwrap();
    let path = path.to_string_lossy().into();

    BackingData { tree, input, path }
}

include!(concat!(env!("OUT_DIR"), "/is_container.rs"));

pub fn is_query(kind: &str) -> bool {
    kind == "query"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_read_json() {
        assert!(is_container("resource"));
        assert!(!is_container("resource_type"));
    }
}
