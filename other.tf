resource "aws_ami" "pc" {
  ami = "something_really_cool"
}

resource "aws_rds_instance" "not-my-db" {
  size = "t2.large"
  num  = 14
}
