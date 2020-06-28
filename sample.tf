resource "aws_rds_instance" "my-db" {
  size = "t2.large"
  num  = 12
}

resource "aws_rds_instance" "my-db" {
  num = 12
}
