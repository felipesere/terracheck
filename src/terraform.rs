use tree_sitter::{Language, Parser, Query};

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

include!(concat!(env!("OUT_DIR"), "/is_container.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_read_json() {
        assert!(is_container("resource"));
        assert!(!is_container("resource_type"));
    }
}
