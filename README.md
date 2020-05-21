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


## Allow
\`\`\`
resource "aws_db_instance" * {
  engine = "postgres"
}
\`\`\`


## Deny
\`\`\`
resource "aws_db_instance" * {
}
\`\`\`


...possibly more allow/deny blocks...

```

We'd run throught the terraform code looking for matches resources that match
either `## Allow` or `## Deny` blocks and then flag as apropriate.

There would be some mechanism for describing placeholders and constraints on attributes names and connections.

## What we could use

It would be fun to try and lift terraform files to AST and then match that against
a representation of the rules.

This could be useful to parse terraform [tree-sitter](https://github.com/tree-sitter/tree-sitter)
