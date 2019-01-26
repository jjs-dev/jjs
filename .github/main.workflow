workflow "OnCommitCheck" {
  on = "push"
  resolves = ["Check"]
}

action "Check" {
  uses = "docker://mikailbag/jjs-dev:jjs-dev"
  runs = "bash ./scripts/ci.sh"
}
