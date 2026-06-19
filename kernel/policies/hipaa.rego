package ucser.hipaa

default allow = false

allow {
    not contains_violation
}

contains_violation {
    input.cmd == "rm -rf"
}

contains_violation {
    contains(input.cmd, "LD_PRELOAD")
}
