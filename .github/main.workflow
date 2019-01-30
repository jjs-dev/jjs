workflow "OnCommitCheck" {
  on = "push"
  resolves = ["Package"]
}

action "Check" {
  uses = "docker://mikailbag/jjs-dev:jjs-dev"
  runs = "bash ./scripts/ci.sh"
}

action "Package" {
  uses = "docker://mikailbag/jjs-dev:jjs-dev"
  needs = ["Check"]
  runs = "cargo run --bin devtool -- Pkg"
}
