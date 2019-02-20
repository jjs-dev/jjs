#!/bin/bash

mkdir -p "$1"
mkdir "$1/"{bin,etc,lib,tmp,var}
cat > "$1/etc/jjs.toml" << EOF
toolchain-root= "/opt/jjs-tc/root"
EOF
mkdir "$1/etc/toolchains"
cat > "$1/etc/toolchains/cpp.toml" << EOF
name="cpp"
suffix="cpp"
[[build]]
argv=["/usr/bin/g++", "\$(System.SourceFilePath)", "-o", "\$(System.BinaryFilePath)", "-std=c++17", "-Wall", "-Wextra", "-Wpedantic", "-DJJS"]
[run]
argv=["\$(System.BinaryFilePath)"]
EOF
