use pulldown_cmark::{
    Event::{Start, Text},
    Parser,
    Tag::{CodeBlock, Heading},
};
use tree_sitter::Node;

use super::terraform;
use ast::AST;
use regex::Regex;
use rule::{Decision, Rule};
use std::fmt::{self, write, Write};
use std::io::Read;

mod ast;
mod rule;

lazy_static! {
    static ref RE: Regex = Regex::new(r#"\$\((?P<operation>[^)]+)\)"#).unwrap();
}

#[derive(Debug)]
pub struct Document {
    title: String,
    pub rules: Vec<Rule>,
}

impl Document {
    pub fn matches<R: Read>(&self, mut terraform: R) -> bool {
        let mut parser = crate::terraform::parser();
        let mut content = String::new();
        terraform
            .read_to_string(&mut content)
            .expect("unable to read terraform code");
        let terraform_ast = parser.parse(&content, None).unwrap();

        for rule in &self.rules {
            if !rule.matches(&terraform_ast, &content) {
                return false;
            }
        }

        true
    }
}

trait Predicate: std::fmt::Debug {
    fn check(&self) -> bool;
}

#[derive(Debug)]
struct Or {
    capture: String,
    options: Vec<String>,
}

impl Predicate for Or {
    fn check(&self) -> bool {
        self.options.contains(&self.capture)
    }
}

#[derive(Debug)]
struct True;

impl Predicate for True {
    fn check(&self) -> bool {
        return true;
    }
}

trait ToSexp {
    fn to_sexp(&self, output: &mut dyn Write) -> fmt::Result;
}

impl ToSexp for Vec<Query> {
    fn to_sexp(&self, output: &mut dyn Write) -> fmt::Result {
        self.iter().try_for_each(|op| {
            op.to_sexp(output)?;
            write!(output, " ")
        })
    }
}

// Coudl this just be values.join(" ")?
fn join(values: &[String]) -> String {
    values
        .iter()
        .map(|val| format!("{:?}", val))
        .collect::<Vec<String>>()
        .join(" ")
}

impl ToSexp for Query {
    fn to_sexp(&self, output: &mut dyn Write) -> fmt::Result {
        match self {
            Query::Unknown {
                operation,
                reference,
            } => write(output, format_args!("(#{}? @{})", operation, reference)),
            Query::Eq { values, reference } => write(
                output,
                format_args!(
                    "(#eq? @{reference} {value})",
                    reference = reference,
                    value = join(values),
                ),
            ),
            Query::Match { values, reference } => write(
                output,
                format_args!(
                    "(#match? @{reference} {value})",
                    reference = reference,
                    value = join(values),
                ),
            ),
            Query::Or { values, reference } => write(
                output,
                format_args!(
                    "(#or? @{reference} {value})",
                    reference = reference,
                    value = join(values),
                ),
            ),
        }
    }
}

#[derive(Debug)]
enum Query {
    Eq {
        reference: String,
        values: Vec<String>,
    },
    Match {
        reference: String,
        values: Vec<String>,
    },
    Or {
        reference: String,
        values: Vec<String>,
    },
    Unknown {
        reference: String,
        operation: String,
    },
}

pub struct Reference {
    chars: Box<dyn Iterator<Item = char>>,
}

impl Reference {
    fn new() -> Self {
        Reference {
            chars: Box::new("abcdefghijklmnopqrstuvwxyz".chars()),
        }
    }

    fn next(&mut self) -> String {
        self.chars.next().unwrap().to_string()
    }
}

fn children<'a>(node: &'a Node) -> Vec<Node<'a>> {
    let mut nodes = Vec::new();
    for n in node.children(&mut node.walk()) {
        nodes.push(n)
    }

    nodes
}

enum NodeKind<'a> {
    Unnamed,
    Query {
        value: String,
    },
    Container {
        kind: String,
        children: Vec<Node<'a>>,
    },
    Other {
        kind: String,
        value: String,
    },
}

fn kind<'a>(node: &'a Node, source: &str) -> NodeKind<'a> {
    if !node.is_named() {
        return NodeKind::Unnamed;
    }

    let kind: String = node.kind().into();
    let value: String = node.utf8_text(source.as_bytes()).unwrap().into();

    if terraform::is_query(&kind) {
        NodeKind::Query { value }
    } else if terraform::is_container(&kind) {
        NodeKind::Container {
            kind,
            children: children(&node),
        }
    } else {
        NodeKind::Other { kind, value }
    }
}

fn ast(node: Node, source: &str, generator: &mut Reference) -> (Option<AST>, Vec<Query>) {
    match kind(&node, source) {
        NodeKind::Unnamed => (None, Vec::new()),
        NodeKind::Query { value } => prcoess_query(value, generator),
        NodeKind::Container { kind, children } => {
            let mut queries = Vec::new();
            let mut children_ast: Vec<Box<AST>> = Vec::new();
            for child in children {
                match ast(child, &source, generator) {
                    (None, _) => continue,
                    (Some(ast), mut new_queries) => {
                        children_ast.push(Box::new(ast));
                        queries.append(&mut new_queries);
                    }
                }
            }
            return (
                Some(AST::Container {
                    kind,
                    children: children_ast,
                }),
                queries,
            );
        }
        NodeKind::Other { kind, value } => {
            let reference = generator.next();
            return (
                Some(AST::Fixed {
                    kind,
                    reference: reference.clone(),
                }),
                vec![Query::Eq {
                    reference: reference,
                    values: vec![value],
                }],
            );
        }
    }
}

fn prcoess_query(value: String, generator: &mut Reference) -> (Option<AST>, Vec<Query>) {
    let caps = RE.captures(&value).unwrap();

    let operation: String = caps["operation"].trim().to_string();
    if operation == "*" {
        return (Some(AST::Any), Vec::new());
    }

    let reference = generator.next();
    // This will need extracting into its own module with a small parser for operations
    if operation.contains("||") {
        let caps = Regex::new("(?P<left>[^ ]+) \\|\\| (?P<right>.+)")
            .unwrap()
            .captures(&operation)
            .unwrap();
        let left = caps["left"].trim().to_string();
        let right = caps["right"].trim().to_string();

        return (
            Some(AST::Referenced {
                reference: reference.clone(),
            }),
            vec![Query::Or {
                reference,
                values: vec![left, right],
            }],
        );
    }

    return (
        Some(AST::Referenced {
            reference: reference.clone(),
        }),
        vec![Query::Unknown {
            reference,
            operation: operation,
        }],
    );
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

    #[test]
    fn turns_a_rule_into_s_expression() {
        let r = Rule {
            title: "Example".into(),
            code: r#"
                    resource "aws_rds_instance" $(*) {
                        size = $(*)
                    }
                    "#
            .into(),
            decision: Decision::Allow,
        };

        let mut buffer = String::new();
        r.to_sexp(&mut buffer).unwrap();

        assert_eq!(
            r#"((configuration (resource (resource_type) @a (*) (block (attribute (identifier) @b (*) ) ) ) @result )(#eq? @a "\"aws_rds_instance\"") (#eq? @b "size") )"#,
            buffer
        )
    }
}
