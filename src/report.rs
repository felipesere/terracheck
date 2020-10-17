use serde::Serialize;
use crate::terraform::BackingData;
use std::collections::HashMap;
use std::io::Write;
use tinytemplate::{format_unescaped, TinyTemplate};

use crate::document::rule::{Decision, MatchResult};

static TEMPLATE : &'static str = r#"{{ for value in success }}
{value} ... ✅
{{ endfor }}
{{ for failure in failures }}
{failure.file} ... ❌
{failure.code}
{{ endfor }}"#;

type NodeId = usize;

#[derive(Debug, Serialize)]
struct Failure {
    file: String,
    code: String,
}

#[derive(Debug, Default, Serialize)]
struct Context {
    failures: Vec<Failure>,
    success: Vec<String>,
}

pub struct StdoutReport<'a, W: Write> {
    output: W,
    template: TinyTemplate<'a>,
}

impl <'a, W: Write> StdoutReport<'a, W> {
    pub fn new(output: W) -> Self {
        let mut template = TinyTemplate::new();
        template.set_default_formatter(&format_unescaped);
        template.add_template("success_and_failure", TEMPLATE).unwrap();
        StdoutReport { output, template }
    }
}

/// Present the results to a user in a meaningful way
pub trait Report {
    fn about(&mut self, terraform: &BackingData, match_results: Vec<MatchResult>);
}

impl <'a, W: Write> Report for StdoutReport<'a, W> {
    // This needs to a single call, not a giant loop...
    fn about(&mut self, terraform: &BackingData, match_results: Vec<MatchResult>) {
        let mut context = Context::default();
        if match_results.is_empty() {
            context.success.push(terraform.path.clone());
        }

        let mut results_for_node: HashMap<NodeId, Vec<MatchResult>> = HashMap::new();

        for m in match_results {
            let resources = results_for_node.entry(m.node_info.id).or_insert(Vec::new());
            resources.push(m);
        }

        for results in results_for_node.values() {
            let any_allow = results.iter().any(|m| m.decision == Decision::Allow);
            if !any_allow {
                let dennial = results.iter().find(|m| m.decision == Decision::Deny).unwrap();

                context.failures.push(Failure {
                    file: terraform.path.clone(),
                    code: terraform.text_range(&dennial.node_info.byte_range).to_string(),
                });
            } else {
                context.success.push(terraform.path.clone())
            }
        }
        let rendered = self.template.render("success_and_failure", &context).unwrap();
        write!(self.output, "{}", rendered).expect("TODO: should we lift this?");
    }
}
