use crate::document::ToSexp;

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
    fn to_sexp(&self) -> String {
        match self {
            AST::Any => "(*)".into(),
            AST::Container { kind, children } => {
                let inner = children
                    .iter()
                    .map(|child| child.to_sexp())
                    .collect::<Vec<String>>()
                    .join(" ");

                if kind == "resource" {
                    format!("({} {}) @result", kind, inner)
                } else {
                    format!("({} {})", kind, inner)
                }
            }
            AST::Fixed { kind, reference: r } => {
                format!("({kind}) @{reference}", kind = kind, reference = r)
            }
            AST::WithQuery { reference } => format!("(*) @{}", reference),
        }
    }
}
