# Welcome To Terracheck

No, this does not exist yet, but is barely an idea.

The idea of `terracheck` is to be able to run a set of rules
against folder containing a bunch of terraform files and modules
and have it report and drift it detects.

The rules should roughly follow the the pattern of terraform definitions
itself, to make it obvious what would be allowed/denied.

As sample rule could be
> We only want PostgreSQL RDS instances

and then there could be a markdown document saying something like

```markdown
nr: 1235
---
# Only allow PostgreSQL RDS instances

Some clever rationale comparing the driver support to MySQL and Oracle


## Allow - Major PSQL version
\`\`\`
resource "aws_db_instance" * {
  engine = "postgres"
  engine_version = XX.YY
}
\`\`\`

## Allow - Major MySQL version
\`\`\`
resource "aws_db_instance" * {
  engine = "mysql"
  engine_version = XX.YY
}
\`\`\`

## Deny - any other DB engine

\`\`\`
resource "aws_db_instance" * {
}
\`\`\`

...possibly more allow/deny blocks...

## Exceptions

* module.warehouse.aws_db_instance.main
* id-123u04r902380
* path/to/file.tf

```

We'd run throught the terraform code looking for matches resources that match
either `## Allow` or `## Deny` blocks and then flag as apropriate.

There would be some mechanism for describing placeholders and constraints on attributes names and connections.

## What we could use

It would be fun to try and lift terraform files to AST and then match that against
a representation of the rules.

We can use [tree-sitter](https://github.com/tree-sitter/tree-sitter) to parse Terraform files into AST
and run queries against it.

For example, if we had the following terraform:

```terraform
resource "aws_lb" "main" {
  name               = var.service_name
  subnets            = var.public_subnet_ids
  security_groups    = [aws_security_group.main.id]
  idle_timeout       = 900
  load_balancer_type = "application"

  enable_deletion_protection = false

  access_logs {
    bucket  = var.access_logs_bucket_name
    enabled = true
    prefix  = var.service_name
  }
}
```

it would be nice if the following rule matched it:

```
resource "aws_lb" * {
  enable_deletion_protection = false
}
```

The semantically equivalent query in `tree-sitter` looks like this:

```lisp
(
  (resource
    (resource_type) @type
    (block
      (attribute
        (identifier) @id
        (boolean) @val
  )))
  (#eq? @type "\"aws_lb\"")
  (#eq? @id "enable_deletion_protection")
  (#eq? @val true)
) @result
```

## Valuable resources

These tests describe possible queries quite nicely:
[query_test.rs](https://github.com/tree-sitter/tree-sitter/blob/deeeb67a3b20043e05b7197022aa285fa6b1b58c/cli/src/tests/query_test.rs)