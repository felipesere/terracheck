use tree_sitter::{Language, Parser, Query, Tree};

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

// Considering a type for Tree+Text?
pub fn parse(input: &str) -> (Tree, &str) {
    let mut parser = parser();

    let tree = parser.parse(&input, None).unwrap();

    (tree, input)
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
