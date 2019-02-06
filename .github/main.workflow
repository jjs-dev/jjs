workflow "OnPush" {
  on = "push"
  resolves = ["Check"]
}

action "Check" {
  uses = "docker://mikailbag/jjs-dev:latest"
  needs = ["Upload_devel_image"]
  runs = "bash ./scripts/ci.sh"
}

action "Publish" {
  uses = "docker://mikailbag/jjs-dev:latest"
  needs = ["Check"]
  runs = "bash ./scripts/publish.sh"
  secrets = ["JJS_DEVTOOL_YANDEXDRIVE_ACCESS_TOKEN"]
}

action "Docker_login" {
  uses = "actions/docker/login@master"
  secrets = [
    "DOCKER_PASSWORD",
    "DOCKER_USERNAME",
  ]
}

action "Build_devel_image" {
  uses = "actions/docker/cli@c08a5fc9e0286844156fefff2c141072048141f6"
  needs = ["Docker_login"]
  args = "build -t mikailbag/jjs-dev . -f ./scripts/ci.Dockerfile"
}

action "Upload_devel_image" {
  uses = "actions/docker/cli@c08a5fc9e0286844156fefff2c141072048141f6"
  args = "push mikailbag/jjs-dev:latest"
  needs = ["Build_devel_image"]
}
