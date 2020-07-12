use crate::terraform;
use pulldown_cmark::{
    Event::{Start, Text},
    Parser,
    Tag::{CodeBlock, Heading},
};
use rule::{Decision, MatchResult, Rule};
use std::io::Read;

mod ast;
mod rule;

#[derive(Debug)]
pub struct Document {
    title: String,
    pub rules: Vec<Rule>,
}

impl Document {
    pub fn matches<R: Read>(&self, mut source: R) -> bool {
        let mut content = String::new();

        source
            .read_to_string(&mut content)
            .expect("unable to read terraform code");
        let backing_data = terraform::parse(&content);

        self
            .rules
            .iter()
            .flat_map(|r| r.matches(&backing_data))
            .any(|m| match m {
            MatchResult::Matched {
                decision: Decision::Allow,
                node_info: _,
                title: _,
            } => true,
            _ => false,
        })
    }
}

pub fn from_reader<R: Read>(mut input: R) -> Option<Document> {
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
    use pretty_assertions::assert_eq;

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
}
