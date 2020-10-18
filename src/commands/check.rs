use argh::FromArgs;
use glob::glob;
use std::fs::File;
use std::path::PathBuf;

use crate::Run;
use report::{Report, StdoutReport};
use terraform::BackingData;

#[derive(FromArgs)]
/// Verifies if any terraform resource matches the rule in the markdown file
#[argh(subcommand, name = "check")]
pub struct Check {
    #[argh(positional)]
    path: PathBuf,
}

fn paths_in(path: &str) -> Vec<PathBuf> {
    glob(path)
        .expect("Failed to read glob pattern")
        .filter_map(Result::ok)
        .collect()
}

impl Run for Check {
    fn run(self) {
        let mut report = StdoutReport::new(std::io::stdout());
        let files = if self.path.is_dir() {
            let pattern = format!("{}/*.md", self.path.to_string_lossy());
            paths_in(&pattern)
        } else {
            vec![self.path]
        };

        let tf_files_to_check: Vec<BackingData> = paths_in("**/*.tf")
            .into_iter()
            .map(terraform::parse)
            .collect();

        for path in files {
            let file = File::open(path).expect("could not open rule file");
            let rule = document::from_reader(&file).expect("was not able to parse markdown");

            for backing_data in tf_files_to_check.iter() {
                report.about(backing_data, rule.matches(&backing_data));
            }
        }
    }
}
