use argh::FromArgs;
use colored::*;
use glob::glob;
use std::fs::read_to_string;

#[derive(FromArgs)]
/// Prints everything that was parsed
#[argh(subcommand, name = "show")]
pub struct Show {
    /// whether to show only errors
    #[argh(switch, short = 'e')]
    errors: bool,
}

impl crate::Run for Show {
    fn run(self) {
        let mut parser = crate::terraform::parser();
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
                        } else if !self.errors {
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
}
