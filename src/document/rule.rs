use super::ast;
use crate::document::{Reference, ToSexp};
use crate::terraform;
use std::fmt::{self, Write};

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
