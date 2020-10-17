#![allow(dead_code)]

use argh::FromArgs;
use commands::Check;
use commands::Show;

mod commands;
mod report;

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
        let doc = document::from_reader(doc_source.as_bytes()).expect("unable to create document");

        let tf = terraform::parse(tf);

        doc.matches(&tf)
            .iter()
            .any(|m| m.decision == Decision::Allow)
    }
}
