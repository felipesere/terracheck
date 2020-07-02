use tree_sitter::{QueryPredicate, QueryPredicateArg};

pub trait Predicate: std::fmt::Debug {
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

pub fn read_operation<F: Fn(u32) -> String>(
    query_pred: &QueryPredicate,
    node_value: F,
) -> Box<dyn Predicate> {
    let capture = capture_from(query_pred, node_value);
    let options = values_from(query_pred);
    return match query_pred.operator.as_ref() {
        "or?" => Box::new(Or {
            capture: capture.unwrap(),
            options,
        }),
        _ => Box::new(True {}) as Box<dyn Predicate>,
    };
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
