workflow "OnPush" {
  on = "push"
  resolves = ["Docs", "Test"]
}

action "Check" {
  uses = "docker://mikailbag/jjs-dev:gh-70feb585b6ab9b738c3abe984e83cb81588e89f5"
  runs = "cargo build-jjs"
  env = {
    RUST_BACKTRACE = "1"
  }
}

action "Test" {
  uses = "docker://mikailbag/jjs-dev:gh-70feb585b6ab9b738c3abe984e83cb81588e89f5"
  runs = "cargo test-jjs"
  needs = ["Check"]
  env = {
      RUST_BACKTRACE = "1"
  }
}

action "Docs" {
  uses = "docker://mikailbag/jjs-dev:gh-70feb585b6ab9b738c3abe984e83cb81588e89f5"
  needs = ["Check"]
  runs = "cargo docs-jjs"
  secrets = ["GITHUB_TOKEN"]
  env = {
    RUST_BACKTRACE = "1"
  }
}
