#!/usr/bin/env bash
## Build and Test PRs for NuttX Kernel and Apps

set -e  ## Stop on error

## Set the GitHub Token. Should have permission to Post PR Comments.
## export GITHUB_TOKEN=...
. $HOME/github-token.sh

## Set the GitLab Token for creating snippets
## export GITLAB_TOKEN=...
. $HOME/gitlab-token.sh

set -x  ## Echo commands

# curl -L \
#   -H "Accept: application/vnd.github+json" \
#   -H "Authorization: Bearer $GITHUB_TOKEN" \
#   -H "X-GitHub-Api-Version: 2022-11-28" \
#   https://api.github.com/notifications/threads/14630615157 \
#   | jq

## Enable Rust Logging
export RUST_LOG=info 
export RUST_BACKTRACE=1

for (( ; ; ))
do
  cargo run
  break
  sleep 300
done
