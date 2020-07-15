use crate::terraform::BackingData;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use crate::document::rule::{Decision, MatchResult};

pub struct Report<W: Write> {
    output: W,
}

type NodeId = usize;

impl<W: Write> Report<W> {
    pub fn to(output: W) -> Self {
        Report { output }
    }

    // TODO: this needs enxtending and improving.
    // There can be
    // * multiple matches for the same resource, but different rules (1 doc == n rules!)
    // * multiple resources within a `path` (or file) mmight have matched
    // * if I want to weigh `Accept` vs `Deny`, they need to match resource (which is node_info.id) and rule (which is title)
    pub fn about(&mut self, path: &Path, terraform: &BackingData, match_results: Vec<MatchResult>) {
        if match_results.is_empty() || match_results.iter().all(|m| m.decision == Decision::Allow) {
            self.output
                .write_all(format!("{:?} ... ✅\n", path).as_bytes())
                .unwrap();
        }

        let mut results_for_node: HashMap<NodeId, Vec<MatchResult>> = HashMap::new();

        for m in match_results {
            let resources = results_for_node.entry(m.node_info.id).or_insert(Vec::new());

            resources.push(m.clone()); // TODO fix this clone someho
        }

        for (_, ms) in results_for_node {
            let any_allow = ms.iter().any(|m| m.decision == Decision::Allow);
            if !any_allow {
                let dennial = ms.iter().find(|m| m.decision == Decision::Deny).unwrap();

                self.output
                    .write_all(format!("{:?} ... ❌\n", path).as_bytes())
                    .unwrap();
                self.output
                    .write(
                        terraform
                            .text_range(&dennial.node_info.byte_range)
                            .as_bytes(),
                    )
                    .expect("unable to write");
                self.output.write("\n".as_bytes()).unwrap();
            }
        }
    }
}
