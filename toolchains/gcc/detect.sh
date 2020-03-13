#!/usr/bin/env bash
if [[ "x$JJS_GCC_SKIP" != "x" ]]; then
    exit 1;
fi
gcc_path=$( command -v gcc )
if [[ -z "$gcc_path" ]]; then
    echo "gcc not found";
    exit 1;
fi
echo "found gcc at $gcc_path"
echo "set-env:GCC=$gcc_path" >> "$1"
