use pulldown_cmark::{
    Event::{Start, Text},
    Parser,
    Tag::{CodeBlock, Heading},
};
use tree_sitter::{Node, QueryCursor, QueryPredicateArg};

use super::terraform;
use ast::AST;
use regex::Regex;
use std::io::Read;

mod ast;

#[derive(Debug)]
pub struct Document {
    title: String,
    pub rules: Vec<Rule>,
}

impl Document {
    pub fn matches<R: Read>(&self, mut terraform: R) -> bool {
        let mut cursor = QueryCursor::new();
        let mut parser = crate::terraform::parser();
        let mut content = String::new();
        terraform
            .read_to_string(&mut content)
            .expect("unable to read terraform code");
        let terraform_ast = parser.parse(&content, None).unwrap();
        let text_callback = |n: Node| &content[n.byte_range()];

        for rule in &self.rules {
            let query = terraform::query(&rule.to_sexp());

            let mut matches = cursor
                .matches(&query, terraform_ast.root_node(), text_callback)
                .peekable();
            if matches.peek().is_some() {
                let m = matches.next().unwrap();

                let node_value = |idx: u32| {
                    let node = m
                        .captures
                        .iter()
                        .find_map(|cap| {
                            if cap.index == idx {
                                Some(cap.node)
                            } else {
                                None
                            }
                        })
                        .expect(
                            "capture of index was not in the list of expected captures of query",
                        );
                    &content[node.byte_range()]
                };

                for predicate in query.general_predicates(m.pattern_index) {
                    for arg in &predicate.args {
                        match arg {
                            QueryPredicateArg::String(s) => println!("literal: {}", s),
                            QueryPredicateArg::Capture(capture_ref) => {
                                println!("captured: {}", node_value(*capture_ref))
                            }
                        }
                    }
                }
                return true;
            }
        }
        false
    }
}

#[derive(Eq, PartialEq, Debug)]
pub enum Decision {
    Allow,
    Deny,
}

fn sexp(queries: Vec<Query>) -> String {
    queries
        .iter()
        .map(|query| match &query.value {
            None => format!(
                "(#{op}? @{reference})",
                op = query.operation,
                reference = query.reference
            ),
            Some(val) => format!(
                "(#{op}? @{reference} {value})",
                op = query.operation,
                reference = query.reference,
                value = format!("{:?}", val)
            ),
        })
        .collect::<Vec<String>>()
        .join(" ")
}

#[derive(Debug)]
struct Query {
    reference: String,
    operation: String,
    value: Option<String>,
}

#[derive(Debug)]
pub struct Rule {
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

    pub fn to_sexp(&self) -> String {
        let mut parser = terraform::parser();

        let tree = parser.parse(&self.code, None).unwrap();

        let ast = ast(tree.root_node(), self.code.as_str(), &mut Reference::new());

        if let (Some(nodes), queries) = ast {
            format!(
                "({nodes} {query})",
                nodes = nodes.sexp(),
                query = sexp(queries)
            )
        } else {
            "".into()
        }
    }
}

struct Reference {
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

fn ast(node: Node, source: &str, generator: &mut Reference) -> (Option<AST>, Vec<Query>) {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#"\$\((?P<operation>[^)]+)\)"#).unwrap();
    }

    let mut queries = Vec::new();
    if !node.is_named() {
        return (None, queries);
    }

    let kind: String = node.kind().into();
    let value: String = node.utf8_text(source.as_bytes()).unwrap().into();

    if kind == "query" {
        let caps = RE.captures(&value).unwrap();

        let operation: String = caps["operation"].trim().to_string();
        if operation == "*" {
            return (Some(AST::Any), queries);
        }

        let reference = generator.next();
        return (
            Some(AST::WithQuery {
                reference: reference.clone(),
            }),
            vec![Query {
                reference,
                operation: operation.into(),
                value: None,
            }],
        );
    }

    if terraform::is_container(&kind) {
        let mut children: Vec<Box<AST>> = Vec::new();
        for child in node.children(&mut node.walk()) {
            match ast(child, &source, generator) {
                (None, _) => continue,
                (Some(ast), mut new_queries) => {
                    children.push(Box::new(ast));
                    queries.append(&mut new_queries);
                }
            }
        }
        (Some(AST::Container { kind, children }), queries)
    } else {
        let reference = generator.next();
        queries.push(Query {
            reference: reference.clone(),
            operation: "eq".into(), // enum here?
            value: Some(value),
        });
        (Some(AST::Fixed { kind, reference }), queries)
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

        assert_eq!(
            r#"((configuration (resource (resource_type) @a (*) (block (attribute (identifier) @b (*)))) @result) (#eq? @a "\"aws_rds_instance\"") (#eq? @b "size"))"#,
            r.to_sexp()
        )
    }
}
