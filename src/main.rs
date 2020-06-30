#![allow(dead_code)]

use clap::{App, Arg};
use colored::*;
use glob::glob;
use std::fs::read_to_string;
use tree_sitter::Parser;

mod document;
mod terraform;

#[macro_use]
extern crate lazy_static;

fn main() {
    let matches = App::new("My Super Program")
        .version("0.1")
        .about("Checks terraform files for patterns")
        .subcommand(
            App::new("show")
                .arg(
                    Arg::new("error_only")
                        .about("only print errors for easier debugging")
                        .long("--errors")
                        .short('e')
                        .required(false),
                )
                .about("Prints everythinng that was parsed"),
        )
        .subcommand(
            App::new("check").arg(
                Arg::new("rule_file")
                    .about("Verifies if any resource matches the rule in the markdown file")
                    .value_name("RULE_FILE")
                    .takes_value(true)
                    .required(true),
            ),
        )
        .get_matches();

    let parser = terraform::parser();

    match matches.subcommand() {
        ("show", Some(show_matches)) => parse_all(parser, show_matches.is_present("error_only")),
        ("check", Some(check_matches)) => run_check(check_matches.value_of("rule_file").unwrap()),
        _ => println!("Unknown command"),
    }
}

fn run_check(rule_file: &str) {
    use std::fs::File;

    let file = File::open(rule_file).expect("could not open rule file");

    let doc = document::from_reader(&file).expect("was not able to parse markdown");

    for entry in glob("**/*.tf").expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let terraform_content = read_to_string(&path).unwrap();

                if doc.matches(terraform_content.as_bytes()) {
                    println!("{}\n", path.to_str().unwrap());
                }
            }
            err => println!("error: {:?}", err),
        }
    }
}

fn parse_all(mut parser: Parser, only_errors: bool) {
    for entry in glob("**/*.tf").expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let content = read_to_string(&path).unwrap();
                println!("*****");
                println!("{}", path.to_str().unwrap().blue());

                let tree = parser.parse(&content, None).unwrap();

                let mut cursor = tree.root_node().walk();
                // Skip (comfiguration)
                cursor.goto_first_child();
                loop {
                    let node = cursor.node();

                    if node.has_error() {
                        println!("{}", node.utf8_text(&content.as_bytes()).unwrap().red());
                        println!("{}", node.to_sexp().red());
                        println!("");
                    } else if !only_errors {
                        println!("{}", node.utf8_text(&content.as_bytes()).unwrap());
                        println!("{}", node.to_sexp());
                        println!("");
                    }

                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }

                println!("");
            }
            Err(e) => println!("{:?}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_rds() {
        let terraform_content = r#"
resource "aws_rds_instance" "my-db" {
  size = "t2.large"
  num  = 12
}
        "#;

        let document = r#"
# Only allow RDS with an explicit size

Some fancy reason why this matters

## Allow: RDS with a size property set

```
resource "aws_rds_instance" $(*) {
  size = $(somethings)
}
```
        "#;

        // made to fail to see the output
        assert!(matches(terraform_content, document))
    }

    fn matches(tf: &str, doc_source: &str) -> bool {
        let doc = document::from_reader(doc_source.as_bytes()).expect("unable to create document");

        doc.matches(tf.as_bytes())
    }
}
