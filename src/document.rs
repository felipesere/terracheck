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

#[derive(Debug)]
enum AST {
    Container {
        kind: String,
        children: Vec<Box<AST>>,
    },
    Fixed {
        kind: String,
        value: String,
    },
    WithQuery {
        reference: String,
    },
}

impl AST {
    pub fn sexp(&self) -> String {
        match self {
            AST::Container { kind, children } => format!(
                "({} {})",
                kind,
                children
                    .iter()
                    .map(|child| child.sexp())
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
            AST::Fixed {
                kind,
                value: _value,
            } => format!("({kind})", kind = kind),
            AST::WithQuery { reference } => format!("(*) @{reference}", reference = reference),
        }
    }
}

fn sexp(queries: Vec<Query>) -> String {
    queries
        .iter()
        .map(|query| {
            format!(
                "(#match? @{reference} {value})",
                reference = query.reference,
                value = "*"
            )
        })
        .collect::<Vec<String>>()
        .join(" ")
}

#[derive(Debug)]
struct Query {
    reference: String,
    operation: String,
}

struct Pattern {
    nodes: Vec<AST>,
    query: Vec<Query>,
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

        let ast = ast(tree.root_node(), self.code.as_str());

        if let (Some(nodes), queries) = ast {
            format!(
                "({nodes} {query}) @result",
                nodes = nodes.sexp(),
                query = sexp(queries)
            )
        } else {
            "".into()
        }
    }
}

fn ast(node: Node, source: &str) -> (Option<AST>, Vec<Query>) {
    let mut queries = Vec::new();
    if !node.is_named() {
        return (None, queries);
    }

    let kind: String = node.kind().into();

    if kind == "query" {
        let reference: String = "a".into();
        return (
            Some(AST::WithQuery {
                reference: reference.clone(),
            }),
            vec![Query {
                reference,
                operation: node.utf8_text(source.as_bytes()).unwrap().into(),
            }],
        );
    }

    if terraform::is_container(&kind) {
        let mut children: Vec<Box<AST>> = Vec::new();
        for child in node.children(&mut node.walk()) {
            match ast(child, &source) {
                (None, _) => continue,
                (Some(x), mut new_queries) => {
                    children.push(Box::new(x));
                    queries.append(&mut new_queries);
                }
            }
        }
        (Some(AST::Container { kind, children }), queries)
    } else {
        (
            Some(AST::Fixed {
                kind,
                value: node.utf8_text(source.as_bytes()).unwrap().into(),
            }),
            queries,
        )
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

    #[test]
    fn turns_a_rule_into_s_expression() {
        let r = Rule {
            title: "Example".into(),
            code: r#"
                    resource "aws_db_instance" $(*) {
                        engine = $(*)
                    }
                    "#
            .into(),
            decision: Decision::Allow,
        };

        assert_eq!(
            r#"((resource (resource_type) @type) (#eq? @type "\"aws_db_instance\"")) @result"#,
            r.to_sexp()
        )
    }
}
