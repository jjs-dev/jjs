---
name: Bug report
about: Tell about bug in JJS
title: ''
labels: T-bug, T-new
assignees: ''
---

# Bug description
<!-- Describe bug.
Possible items:
  - Expected behavior.
  - Actual behavior.
  - Explanation, additional background, etc. -->

# Steps to reproduce
<!-- Write how this bug can be reproduced.
Ideally, attach script that triggers this bug on fresh JJS instance. -->

# Environment
<!-- Feel free to remove unapplicable lines, or provide additional info.
You can use provided commands to find out information. -->
**OS**: `uname -a`
**JJS version**: `git rev-parse HEAD`
**Rustc version**: `rustc --version`
**Docker version**: `docker --version`, `podman --version`
**Cgroup version**: `stat /sys/fs/cgroup --file-system`
**Invoker permissions**: (Root | Rootless)

# Reminder [delete when creating issue]
If bug is related to sandbox, syscall dump can be very helpful.
You can use following command to obtain it:
`strace -f -o /tmp/jjs-syscall-log.txt -s 128`