use serde::Serialize;
use crate::terraform::BackingData;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use tinytemplate::{format_unescaped, TinyTemplate};

use crate::document::rule::{Decision, MatchResult};

pub struct Report<W: Write> {
    output: W,
}

// failure: "{:?} ... \n", path
//          "{:?} {..code ... }
//
//
static TEMPLATE : &'static str = r#"{{ for value in success }}
{value} ... ✅
{{ endfor }}
{{ for failure in failures }}
{failure.file} ... ❌
{failure.code}
{{ endfor }}"#;

type NodeId = usize;

#[derive(Serialize)]
struct Failure {
    file: String,
    code: String,
}

#[derive(Default, Serialize)]
struct Context {
    failures: Vec<Failure>,
    success: Vec<String>,
}

impl<W: Write> Report<W> {
    pub fn to(output: W) -> Self {
        Report { output }
    }

    // This needs to a single call, not a giant loop...
    pub fn about(&mut self, path: &Path, terraform: &BackingData, match_results: Vec<MatchResult>) {

        let mut template = TinyTemplate::new();
        template.set_default_formatter(&format_unescaped);
        template.add_template("hello", TEMPLATE).unwrap();

        let mut context = Context::default();
        if match_results.is_empty() || match_results.iter().all(|m| m.decision == Decision::Allow) {
            context.success.push(path.to_str().unwrap().into());
        }

        let mut results_for_node: HashMap<NodeId, Vec<MatchResult>> = HashMap::new();

        for m in match_results {
            let resources = results_for_node.entry(m.node_info.id).or_insert(Vec::new());

            resources.push(m.clone()); // TODO fix this clone somehow
        }

        for (_, ms) in results_for_node {
            let any_allow = ms.iter().any(|m| m.decision == Decision::Allow);
            if !any_allow {
                let dennial = ms.iter().find(|m| m.decision == Decision::Deny).unwrap();

                context.failures.push(Failure {
                    file: path.to_str().unwrap().into(),
                    code: terraform.text_range(&dennial.node_info.byte_range).to_string(),
                });
            }
        }

        let rendered = template.render("hello", &context).unwrap();
        print!("{}", rendered);
    }
}
