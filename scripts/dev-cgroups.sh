#!/usr/bin/env bash

# configures cgroups v2 for use with invoker
set -e
echo "THIS IS INSECURE SETUP"
echo "NEVER USE IN PRODUCTION"
user="$(whoami)"
root="/sys/fs/cgroup"
echo "Delegating ${root} to ${user}"
sudo chown "$user" "$root"
sudo chmod +w "$root/"
sudo chown "$user" "$root/cgroup.procs"
sudo mkdir -p "$root/jjs"
sudo chown -R "$user" "$root/jjs"
echo "+pids +memory +cpu" | sudo tee "$root/jjs/cgroup.subtree_control"
