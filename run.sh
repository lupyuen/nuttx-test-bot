#!/usr/bin/env bash
## Handle PRs for NuttX Kernel and Apps

## Update the repo
git pull

## Set the GitHub Token
## export GITHUB_TOKEN=...
. $HOME/github-token.sh

## Echo commands
set -x

## Enable Rust Logging
export RUST_LOG=info 
export RUST_BACKTRACE=1

for (( ; ; ))
do
  cargo run -- --owner apache --repo nuttx
  break;
  sleep 300
done
