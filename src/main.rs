#![allow(dead_code)]

use clap::{App, Arg};
use colored::*;
use glob::glob;
use std::fs::read_to_string;
use tree_sitter::{Node, Parser, Query, QueryCursor};

mod document;
mod terraform;

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
        ("check", Some(check_matches)) => {
            run_check(parser, check_matches.value_of("rule_file").unwrap())
        }
        _ => println!("Unknown command"),
    }
}

fn run_check(mut parser: Parser, rule_file: &str) {
    use std::fs::File;

    let file = File::open(rule_file).expect("could not open rule file");

    let doc = document::from_reader(&file).expect("was not able to parse markdown");
    let rule = doc.rules.get(0).unwrap();

    let query = Query::new(parser.language().unwrap(), &rule.to_sexp()).expect("unworkable query");
    let mut cursor = QueryCursor::new();

    for entry in glob("**/*.tf").expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let terraform_content = read_to_string(&path).unwrap();
                let terraform_ast = parser.parse(&terraform_content, None).unwrap();
                let text_callback = |n: Node| &terraform_content[n.byte_range()];
                let mut matches = cursor
                    .matches(&query, terraform_ast.root_node(), text_callback)
                    .peekable();

                if matches.peek().is_none() {
                    continue;
                }

                println!("\n{}", path.to_str().unwrap());
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
