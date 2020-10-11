use crate::document::ast::AST;
use crate::terraform;
use core::ops::Range;
use regex::Regex;
use std::fmt::{self, write, Write};
use std::iter::successors;
use terraform::BackingData;
use tree_sitter::{Node, QueryCursor, QueryPredicate, QueryPredicateArg};

lazy_static! {
    static ref RE: Regex = Regex::new(r#"\$\((?P<operation>[^)]+)\)"#).unwrap();
}

// I might want to swap these
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum Decision {
    Allow,
    Deny,
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: usize,
    pub byte_range: Range<usize>,
}

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub node_info: NodeInfo,
    pub decision: Decision,
    pub title: String,
}

#[derive(Debug)]
pub struct Rule {
    pub title: String,
    pub decision: Decision,
    result_index: u32,
    query: tree_sitter::Query,
}

impl Rule {
    fn to_sexp(code: String, output: &mut dyn Write) -> fmt::Result {
        let mut parser = terraform::parser();

        let tree = parser.parse(&code, None).unwrap();

        let (nodes, queries) = ast(tree.root_node(), code.as_str(), &mut Reference::new());

        write!(output, "(")?;
        nodes.unwrap().to_sexp(output)?;
        queries.to_sexp(output)?;
        write!(output, ")")
    }

    pub(crate) fn new(title: String, decision: Decision, code: String) -> Result<Self, String> {
        let mut rule_as_sexp = String::new();
        Rule::to_sexp(code, &mut rule_as_sexp).expect("TODO: this should be in infallable? I'm writing to a string...");
        let query = terraform::query(&rule_as_sexp);

        match query.capture_names().iter().position(|cap| cap == "result") {
            Some(idx) => Ok(Rule {
                title,
                decision,
                result_index: idx as u32,
                query,
            }),
            None => Err("There was no @result node found".into()),
        }
    }

    pub(crate) fn matches(&self, terraform: &BackingData) -> Vec<MatchResult> {
        let mut cursor = QueryCursor::new();

        cursor
            .matches(&self.query, terraform.root(), |n: Node| terraform.text(n))
            .filter_map(|m| {
                let node =
                    |idx: u32| {
                        m.captures
                    .iter()
                    .find_map(|cap| {
                        if cap.index == idx {
                            Some(cap.node)
                        } else {
                            None
                        }
                    })
                    .expect("capture of index was not in the list of expected captures of query")
                    };
                let node_value = |idx: u32| terraform.text(node(idx)).to_string();

                let all_predicates_match = self
                    .query
                    .general_predicates(m.pattern_index)
                    .iter()
                    .all(|query_pred| query_to_pred(query_pred, node_value).check());

                if all_predicates_match {
                    let result = node(self.result_index as u32);
                    return Some(MatchResult {
                        node_info: NodeInfo {
                            id: result.id(),
                            byte_range: result.byte_range(),
                        },
                        decision: self.decision,
                        title: self.title.clone(),
                    });
                }
                None
            })
            .collect()
    }
}

fn query_to_pred<F: Fn(u32) -> String>(
    query_pred: &QueryPredicate,
    node_value: F,
) -> Box<dyn Predicate> {
    let capture = capture_from(query_pred, node_value);
    let options = values_from(query_pred);
    match query_pred.operator.as_ref() {
        "or?" => Box::new(Or {
            capture: capture.unwrap(),
            options,
        }),
        _ => Box::new(True {}) as Box<dyn Predicate>,
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

/// We need the quotes around thins, as they matter to the matching of tree-sitter
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
    chars: Box<dyn Iterator<Item = String>>,
}

impl Reference {
    fn new() -> Self {
        Reference {
            chars: Box::new(successors(Some(1), |n| Some(n + 1)).map(|n| n.to_string())),
        }
    }

    fn next(&mut self) -> String {
        self.chars.next().unwrap()
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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn turns_a_rule_into_s_expression() {
        let code = r#"
              resource "aws_rds_instance" $(*) {
                 size = $(*)
              }
            "#
        .into();

        let mut buffer = String::new();
        Rule::to_sexp(code, &mut buffer).unwrap();

        assert_eq!(
            r#"((configuration (resource (resource_type) @1 (*) (block (attribute (identifier) @2 (*) ) ) ) @result )(#eq? @1 "\"aws_rds_instance\"") (#eq? @2 "size") )"#,
            buffer
        )
    }

    #[test]
    fn matches_multiple_resources() {
        let r = Rule::new(
            "Example".into(),
            Decision::Allow,
            r#"
            resource "aws_rds_instance" $(*) {
              size = $(*)
            }
            "#
            .into(),
        )
        .unwrap();

        let terraform_text = r#"
         resource "not_rds" $(*) {
         }

         resource "aws_rds_instance" "a" {
             size = 12
         }

         resource "aws_rds_instance" "b" {
             size = 44
         }
        "#;

        let backing_data = terraform::parse(terraform_text);

        let m = r.matches(&backing_data);

        assert_eq!(2, m.len());
    }
}
