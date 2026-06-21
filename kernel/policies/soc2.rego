package ucser.soc2

deny[msg] {
    input.cmd == "sudo"
    msg := "disallowed_command: sudo"
}

deny[msg] {
    input.cmd == "del"
    msg := "disallowed_command: del"
}
