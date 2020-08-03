#!/usr/bin/env bash

REPO="https://${GITHUB_TOKEN}@github.com/${GITHUB_REPOSITORY}.git"
BRANCH='gh-pages'
git init
git config user.name "${GITHUB_ACTOR}"
git config user.email "${GITHUB_ACTOR}@users.noreply.github.com"
git add .
git commit -m "Triggered by ${GITHUB_SHA}"
git push --force "$REPO" master:$BRANCH
