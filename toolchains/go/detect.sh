#!/usr/bin/env bash

go_path=$( command -v go )
if [[ -z "$go_path" ]]; then
    echo "go not found";
    exit 1;
fi
go env -w GOPATH=.
$go_path get -u golang.org/x/tools/go/packages
$go_path run "$DATA/generate.go" > "program.go"
echo "found go at $go_path"
echo "set-env:GO=$go_path" >> "$1"
