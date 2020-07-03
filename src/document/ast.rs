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
    fn to_sexp(&self, output: &mut dyn Write) {
        match self {
            AST::Any => {
                write!(output, "(*)").unwrap();
            }
            AST::Container { kind, children } => {
                write(output, format_args!("({} ", kind)).unwrap();
                children.iter().for_each(|child| {
                    child.to_sexp(output);
                    write!(output, " ").unwrap();
                });
                write!(output, ")").unwrap();

                if kind == "resource" {
                    write!(output, " @result").unwrap();
                };
            }
            AST::Fixed { kind, reference: r } => {
                write(
                    output,
                    format_args!("({kind}) @{reference}", kind = kind, reference = r),
                )
                .unwrap();
            }
            AST::WithQuery { reference } => {
                write(output, format_args!("(*) @{}", reference)).unwrap();
            }
        };
    }
}
