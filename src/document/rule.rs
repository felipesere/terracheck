use super::ast;
use crate::document::{Or, Predicate, Reference, ToSexp, True};
use crate::terraform;
use std::fmt::{self, Write};
use tree_sitter::{Node, QueryCursor, QueryPredicate, QueryPredicateArg, Tree};

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
