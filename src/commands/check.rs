use argh::FromArgs;
use glob::glob;
use std::fs::read_to_string;
use std::fs::File;
use std::path::PathBuf;

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

impl Run for Check {
    fn run(self) {
        let file = File::open(self.path).expect("could not open rule file");

        let doc = crate::document::from_reader(&file).expect("was not able to parse markdown");
        let mut report = Report::to(std::io::stdout());

        for entry in glob("**/*.tf").expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    let terraform_content = read_to_string(&path).unwrap();
                    let tf = terraform::parse(&terraform_content);

                    report.about(&path, &tf, doc.matches(&tf));
                }
                err => println!("error: {:?}", err),
            }
        }
    }
}
