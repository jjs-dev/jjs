workflow "OnCommitCheck" {
  on = "push"
  resolves = ["Check"]
}

action "Check" {
  uses = "docker://rustlang/rust:nightly"
  runs = "bash ./scripts/ci.sh"
}
