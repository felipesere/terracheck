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
    WithQuery {
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
            AST::WithQuery { reference } => write(output, format_args!("(*) @{}", reference)),
        }
    }
}
