use pulldown_cmark::{
    Event::{Start, Text},
    Parser,
    Tag::{CodeBlock, Heading},
};

use super::terraform;
use std::io::Read;
use tree_sitter::Node;

struct Document {
    title: String,
    rules: Vec<Rule>,
}

#[derive(Eq, PartialEq, Debug)]
enum Decision {
    Allow,
    Deny,
}

struct Rule {
    pub title: String,
    pub code: String,
    pub decision: Decision,
}

impl Rule {
    fn empty() -> Self {
        Rule {
            title: "".into(),
            code: "".into(),
            decision: Decision::Deny,
        }
    }

    fn to_sexp(&self) -> String {
        let mut parser = terraform::parser();

        let tree = parser.parse(&self.code, None).unwrap();

        print(tree.root_node(), self.code.as_str());

        "()".into()
    }
}

fn print(node: Node, source: &str) {
    if !node.is_named() {
        return;
    }

    if node.child_count() > 0 {
        for child in node.children(&mut node.walk()) {
            print(child, source)
        }
    } else {
        println!(
            "{} {}",
            node.kind(),
            node.utf8_text(source.as_bytes()).unwrap()
        );
    }
}

fn from_reader<R: Read>(mut input: R) -> Option<Document> {
    let mut buffer = Vec::new();
    input
        .read_to_end(&mut buffer)
        .expect("was not able to read input");

    let content = std::str::from_utf8(&buffer[..]).expect("hi");

    let mut parser = Parser::new(&content);

    let mut doc = Document {
        title: "".into(),
        rules: Vec::new(),
    };

    let mut current_rule = Rule::empty();

    while let Some(event) = parser.next() {
        match event {
            Start(Heading(1)) => {
                doc.title = consume_text(&mut parser).expect("there should have been a doc title");
            }
            Start(Heading(2)) => {
                let title = consume_text(&mut parser).expect("there should have been title text");
                if title.starts_with("Allow") {
                    current_rule.decision = Decision::Allow;
                }
                current_rule.title = title
            }
            Start(CodeBlock(_)) => {
                current_rule.code = consume_text(&mut parser).expect("there was no code");

                doc.rules.push(current_rule);

                current_rule = Rule::empty();
            }
            _ => {}
        }
    }

    Some(doc)
}

fn consume_text(p: &mut Parser) -> Option<String> {
    if let Some(Text(t)) = p.next() {
        return Some(t.into_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_doc_with_two_rules() {
        let text = r#"
# Only allow MySQL rds instances

The above is just a title

## Allow

```
resource "aws_db_instance" $(*) {
  engine = "mysql"
}
```

## Deny

```
resource "aws_db_instance" $(*) {
}
```

"#;

        let doc = from_reader(text.as_bytes()).expect("there should have been a doc");

        assert_eq!(doc.title, "Only allow MySQL rds instances");
        assert_eq!(doc.rules[0].decision, Decision::Allow);
        assert_eq!(doc.rules[1].decision, Decision::Deny);
    }

    #[ignore]
    #[test]
    fn turns_a_rule_into_s_expression() {
        let r = Rule {
            title: "Example".into(),
            code: r#"
                    resource "aws_db_instance" $(*) {
                        foo = [1, 2, 3]
                        bar = "batz"
                    }
                    "#
            .into(),
            decision: Decision::Allow,
        };

        assert_eq!(
            r#"(resource 
            (resource_type) @type
            (#eq? @type "\"aws_db_instance\"")
        )"#,
            r.to_sexp()
        )
    }
}
