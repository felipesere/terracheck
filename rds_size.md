# Only allow RDS with an explicit size

Some fancy reason why this matters

## Allow: RDS with a size property set

```terraform
resource "aws_rds_instance" $(*) {
  num = $(*)
}
```