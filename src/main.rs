#![allow(dead_code)]

use colored::*;
use glob::glob;
use std::fs::read_to_string;
use std::path::PathBuf;
use tree_sitter::Parser;

use argh::FromArgs;

mod document;
mod terraform;

#[macro_use]
extern crate lazy_static;

#[derive(FromArgs)]
/// Checks terraform files for patterns
struct Args {
    #[argh(subcommand)]
    subcommand: Subcommand,
}

#[derive(FromArgs)]
/// Prints everything that was parsed
#[argh(subcommand, name = "show")]
struct Show {
    /// whether to show only errors
    #[argh(switch, short = 'e')]
    errors: bool,
}

#[derive(FromArgs)]
/// Verifies if any terraform resource matches the rule in the markdown file
#[argh(subcommand, name = "check")]
struct Check {
    #[argh(positional)]
    path: PathBuf,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum Subcommand {
    Show(Show),
    Check(Check),
}

fn main() {
    let parser = terraform::parser();
    match argh::from_env::<Args>().subcommand {
        Subcommand::Show(s) => parse_all(parser, s.errors),
        Subcommand::Check(c) => run_check(c.path),
    }
}

fn run_check(rule_file: PathBuf) {
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

    #[test]
    fn matches_or_expression_in_parens() {
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
  num = $(12 || 13)
}
```
        "#;

        assert!(matches(terraform_content, document))
    }

    fn matches(tf: &str, doc_source: &str) -> bool {
        let doc = document::from_reader(doc_source.as_bytes()).expect("unable to create document");

        doc.matches(tf.as_bytes())
    }
}
