use crate::document::ToSexp;
use std::fmt::{write, Write};

#[derive(Debug)]
pub enum AST {
    Container {
        kind: String,
        children: Vec<Box<AST>>,
    },
    Fixed {
        kind: String,
        reference: String,
    },
    Referenced {
        reference: String,
    },
    Any,
}

impl ToSexp for AST {
    fn to_sexp(&self, output: &mut dyn Write) -> std::fmt::Result {
        match self {
            AST::Any => write!(output, "(*)"),
            AST::Container { kind, children } => {
                write(output, format_args!("({} ", kind))?;
                children.iter().try_for_each(|child| {
                    child.to_sexp(output)?;
                    write!(output, " ")
                })?;
                write!(output, ")")?;

                if kind == "resource" {
                    write!(output, " @result")
                } else {
                    Result::Ok(())
                }
            }
            AST::Fixed { kind, reference: r } => write(
                output,
                format_args!("({kind}) @{reference}", kind = kind, reference = r),
            ),
            AST::Referenced { reference } => write(output, format_args!("(*) @{}", reference)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn any_node_gets_turned_into_tree_sitter_astersisk() {
        matches_sexp(AST::Any, "(*)")
    }

    #[test]
    fn container_prints_its_kind_followed_by_its_children() {
        matches_sexp(
            AST::Container {
                kind: "something".into(),
                children: vec![Box::new(AST::Any)],
            },
            "(something (*) )",
        )
    }

    #[test]
    fn resources_get_an_additional_reference_named_result() {
        matches_sexp(
            AST::Container {
                kind: "resource".into(),
                children: vec![Box::new(AST::Any)],
            },
            "(resource (*) ) @result",
        )
    }

    #[test]
    fn nodes_with_a_fixed_value_use_reference_to_match_later() {
        matches_sexp(
            AST::Fixed {
                kind: "resource_type".into(),
                reference: "1".into(),
            },
            "(resource_type) @1",
        )
    }

    #[test]
    fn nodes_with_query_are_asterisk_with_reference_for_later_matching() {
        matches_sexp(
            AST::Referenced {
                reference: "1".into(),
            },
            "(*) @1",
        )
    }

    fn matches_sexp<T: ToSexp>(node: T, sexp: &'static str) {
        let mut buffer = String::new();
        node.to_sexp(&mut buffer)
            .expect("could not write to buffer");

        assert_eq!(&buffer, sexp)
    }
}
