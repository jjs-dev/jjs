workflow "OnPush" {
  on = "push"
  resolves = ["Publish", "Docs"]
}

action "Check" {
  uses = "docker://mikailbag/jjs-dev:latest"
  runs = "bash -c 'just lint'"
  env = {
    RUST_BACKTRACE = "1"
  }
}

action "Publish" {
  uses = "docker://mikailbag/jjs-dev:latest"
  needs = ["Check"]
  runs = "bash ./scripts/publish.sh"
  secrets = ["JJS_DEVTOOL_YANDEXDRIVE_ACCESS_TOKEN"]
  env = {
    RUST_BACKTRACE = "1"
  }
}

action "Docs" {
  uses = "docker://mikailbag/jjs-dev:latest"
  needs = ["Check"]
  runs = "cargo run -p devtool -- man"
  secrets = ["GITHUB_TOKEN"]
  env = {
    RUST_BACKTRACE = "1"
  }
}
