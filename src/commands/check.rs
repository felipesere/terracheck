use argh::FromArgs;
use glob::glob;
use std::fs::read_to_string;
use std::fs::File;
use std::path::PathBuf;
use std::io::Write;

use crate::Run;
use crate::report::Report;

#[derive(FromArgs)]
/// Verifies if any terraform resource matches the rule in the markdown file
#[argh(subcommand, name = "check")]
pub struct Check {
    #[argh(positional)]
    path: PathBuf,
}

impl Run for Check {
    fn run(self) {
        let file = File::open(self.path).expect("could not open rule file");

        let doc = crate::document::from_reader(&file).expect("was not able to parse markdown");
        let mut report = Report::to(std::io::stdout());

        for entry in glob("**/*.tf").expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    let terraform_content = read_to_string(&path).unwrap();

                    report.about(&path, doc.matches(terraform_content.as_bytes()));
                },
                err => println!("error: {:?}", err),
            }
        }
    }
}
