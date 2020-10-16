use argh::FromArgs;
use glob::glob;
use std::fs::read_to_string;
use std::fs::File;
use std::path::PathBuf;

use crate::document;
use crate::report::Report;
use crate::terraform;
use crate::Run;

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
        let files = if self.path.is_dir() {
            let pattern = format!("{}/*.md", self.path.to_string_lossy());
            paths_in(&pattern)
        } else {
            vec![self.path]
        };

        let tf_files_to_check = paths_in("**/*.tf");

        for path in files {
            let file = File::open(path).expect("could not open rule file");
            let rule = document::from_reader(&file).expect("was not able to parse markdown");
            let mut report = Report::to(std::io::stdout());

            for file_to_check in tf_files_to_check.iter() {
                let terraform_content = read_to_string(&file_to_check).unwrap();
                let tf = terraform::parse(&terraform_content);

                report.about(&file_to_check, &tf, rule.matches(&tf));
            }
        }
    }
}
