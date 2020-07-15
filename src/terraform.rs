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

pub struct BackingData<'a> {
    tree: Tree,
    input: &'a str,
}

impl<'a> BackingData<'a> {
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

// Considering a type for Tree+Text?
pub(crate) fn parse(input: &str) -> BackingData {
    let mut parser = parser();

    let tree = parser.parse(&input, None).unwrap();

    BackingData { tree, input }
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
