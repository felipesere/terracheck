use crate::terraform::BackingData;
use pulldown_cmark::{
    Event::{Start, Text},
    Parser,
    Tag::{CodeBlock, Heading},
};
use rule::{Decision, MatchResult, Rule};
use std::io::Read;

mod ast;
// TODO: this needs a better home or some of the types need to be moved out so they are more
// broadly accessible
pub mod rule;

#[derive(Debug)]
pub struct Document {
    title: String,
    pub rules: Vec<Rule>,
}

impl Document {
    pub fn matches(&self, terraform: &BackingData) -> Vec<MatchResult> {
        self.rules
            .iter()
            .flat_map(|r| r.matches(terraform))
            .collect()
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

    let mut current_rule = None;

    while let Some(event) = parser.next() {
        match event {
            Start(Heading(1)) => {
                doc.title = consume_text(&mut parser).expect("there should have been a doc title");
            }
            Start(Heading(2)) => {
                let title = consume_text(&mut parser).expect("there should have been title text");

                let decision = if title.starts_with("Allow") {
                    Decision::Allow
                } else {
                    Decision::Deny
                };
                current_rule = Some((title, decision));
            }
            Start(CodeBlock(_)) => {
                if let Some((title, decision)) = current_rule {
                    let code = consume_text(&mut parser).expect("there was no code");
                    let rule= Rule::new(title, decision, code).expect("TODO");
                    doc.rules.push(rule);
                }

                current_rule = None
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
