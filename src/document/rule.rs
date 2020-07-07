use crate::document::ast::AST;
use crate::terraform;
use regex::Regex;
use std::fmt::{self, write, Write};
use tree_sitter::{Node, QueryCursor, QueryPredicate, QueryPredicateArg, Tree};

lazy_static! {
    static ref RE: Regex = Regex::new(r#"\$\((?P<operation>[^)]+)\)"#).unwrap();
}

#[derive(Eq, PartialEq, Debug)]
pub enum Decision {
    Allow,
    Deny,
}
#[derive(Debug)]
pub struct Rule {
    pub title: String,
    pub code: String,
    pub decision: Decision,
}

impl Rule {
    pub(crate) fn empty() -> Self {
        Rule {
            title: "".into(),
            code: "".into(),
            decision: Decision::Deny,
        }
    }

    pub(crate) fn matches(&self, terraform_ast: &Tree, text: &str) -> bool {
        let mut cursor = QueryCursor::new();
        let mut output = String::new();
        self.to_sexp(&mut output)
            .expect("unable to turn rule into s-exp");
        let query = terraform::query(&output);

        let text_callback = |n: Node| &text[n.byte_range()];

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
                    .expect("capture of index was not in the list of expected captures of query");
                text_callback(node).to_string()
            };

            let funcs: Vec<Box<dyn Predicate>> = query
                .general_predicates(m.pattern_index)
                .iter()
                .map(|query_pred| {
                    let capture = capture_from(query_pred, node_value);
                    let options = values_from(query_pred);
                    return match query_pred.operator.as_ref() {
                        "or?" => Box::new(Or {
                            capture: capture.unwrap(),
                            options,
                        }),
                        _ => Box::new(True {}) as Box<dyn Predicate>,
                    };
                })
                .collect();

            return funcs.iter().all(|func| func.check());
        }
        true
    }
}

fn capture_from<F: Fn(u32) -> String>(
    predicate: &QueryPredicate,
    extract_value: F,
) -> Option<String> {
    for arg in &predicate.args {
        match arg {
            QueryPredicateArg::Capture(cap) => return Some(extract_value(*cap)),
            _ => continue,
        }
    }

    None
}

fn values_from(predicate: &QueryPredicate) -> Vec<String> {
    let mut values = Vec::new();
    for arg in &predicate.args {
        match arg {
            QueryPredicateArg::String(s) => values.push(s.to_string()),
            _ => continue,
        }
    }

    values
}

impl ToSexp for Rule {
    fn to_sexp(&self, output: &mut dyn Write) -> fmt::Result {
        let mut parser = terraform::parser();

        let tree = parser.parse(&self.code, None).unwrap();

        let (nodes, queries) = ast(tree.root_node(), self.code.as_str(), &mut Reference::new());

        write!(output, "(")?;
        nodes.unwrap().to_sexp(output)?;
        queries.to_sexp(output)?;
        write!(output, ")")
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

pub trait ToSexp {
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
