#!/usr/bin/env bash
if [[ "x$JJS_GPP_SKIP" != "x" ]]; then
    exit 1;
fi
gpp_path=$( command -v g++ )
if [[ -z "$gpp_path"  ]]; then
    echo "g++ not found";
    exit 1;
fi;
echo "found g++ at $gpp_path"
echo "set-env:GPP=$gpp_path" >> "$1"
