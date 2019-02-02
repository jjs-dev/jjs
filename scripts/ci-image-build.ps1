#!/usr/bin/env powershell
docker build -f ./scripts/ci.Dockerfile -t mikailbag/jjs-dev:jjs-dev .
#docker push mikailbag/jjs-dev:jjs-dev