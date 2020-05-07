#!/usr/bin/env bash

df -h
# rm -rf "/usr/local/share/boost"
# rm -rf /usr/share/dotnet
apt-get remove -y '^ghc-8.*'
apt-get remove -y '^dotnet-.*'
apt-get remove -y 'php.*'
apt-get remove -y azure-cli google-cloud-sdk hhvm google-chrome-stable firefox powershell mono-devel
apt-get autoremove -y
# rm -rf "$AGENT_TOOLSDIRECTORY"
df -h
