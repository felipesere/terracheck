use tree_sitter::{Language, Parser};

extern "C" {
    fn tree_sitter_terraform() -> Language;
}

pub fn parser() -> Parser {
    let mut parser = Parser::new();
    let language = unsafe { tree_sitter_terraform() };
    parser
        .set_language(language)
        .expect("was not able to create the language");

    parser
}

