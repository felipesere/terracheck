resource "aws_rds_instance" "my-db" {
  tags = {
    "family" = "gladis"
  }
}

resource "aws_rds_instance" "meh-db" {
  tags = {
    "family" = "not-gladis"
  }
}
