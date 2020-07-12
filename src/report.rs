use std::io::{Write};
use std::path::Path;

 use crate::document::rule::MatchResult;

pub struct Report<W: Write> {
    output: W
}

impl <W: Write> Report<W> {
    pub fn to(output: W) -> Self {
        Report { output }
    }

    pub fn about(&mut self, path: &Path, match_results: Vec<MatchResult>) {
        if match_results.is_empty() {
            self.output.write_all(format!("{:?} ... ✅", path).as_bytes()).unwrap();
        }
    }
}
