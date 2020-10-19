#![allow(dead_code)]

use argh::FromArgs;
use commands::Check;
use commands::Show;

mod commands;

#[derive(FromArgs)]
/// Checks terraform files for patterns
struct Args {
    #[argh(subcommand)]
    subcommand: Subcommand,
}

pub trait Run {
    fn run(self);
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum Subcommand {
    Show(Show),
    Check(Check),
}

fn main() {
    match argh::from_env::<Args>().subcommand {
        Subcommand::Show(s) => s.run(),
        Subcommand::Check(c) => c.run(),
    }
}

#[cfg(test)]
mod tests {
    use document::rule::Decision;
    use tempfile::tempdir;

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

## Deny: Any other RDS

```
resource "aws_rds_instance" $(*) {
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
        use std::fs::File;
        use std::io::Write;

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("rule.md");
        let mut file = File::create(&file_path).unwrap();
        write!(&mut file, "{}", doc_source).expect("unabke to write source to temp file");

        let doc = document::from_path(file_path).expect("unable to create document");

        let tf = terraform::parse_text(&tf);

        doc.matches(&tf)
            .iter()
            .any(|m| m.decision == Decision::Allow)
    }
}
