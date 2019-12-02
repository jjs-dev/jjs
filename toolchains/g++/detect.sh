#!/usr/bin/env bash

gpp_path=$( which g++ )
if [[ -z "$gpp_path"  ]]; then
    echo "g++ not found";
    exit 1;
fi;
echo "found g++ at $gpp_path"
echo "set-env:GPP=$gpp_path" >> "$1"
