resource "aws_ami" "pc" {
  ami = "something_really_cool"
}

resource "aws_rds_instance" "not-my-db" {
  num = 14
}
