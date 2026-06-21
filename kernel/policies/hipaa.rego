package ucser.hipaa

deny[msg] {
    input.cmd == "rm -rf"
    msg := "disallowed_command: rm -rf"
}

deny[msg] {
    input.cmd == "rm"
    msg := "disallowed_command: rm"
}

deny[msg] {
    input.env_vars["LD_PRELOAD"]
    msg := "restricted_env: LD_PRELOAD"
}
