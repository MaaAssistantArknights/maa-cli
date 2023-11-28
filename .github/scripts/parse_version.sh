#!/bin/bash

set -e

CARGO_PKG_VERSION=$(yq -r '.package.version' maa-cli/Cargo.toml)
COMMIT_SHA=$(git rev-parse HEAD)

if [ "$GITHUB_EVENT_NAME" == "pull_request" ]; then
  echo "PR detected, marking version as alpha pre-release and skipping publish"
  channel="alpha"
  publish="false"
  VERSION="$CARGO_PKG_VERSION-alpha.$(date +%s)"
elif [ "$GITHUB_EVENT_NAME" == "schedule" ]; then
  echo "Scheduled event detected, marking version as alpha pre-release and publish to alpha channel"
  # check if there are some new commits
  channel="alpha"
  pubulished_commit=$(yq -r ".details.commit" version/$channel.json)
  last_commit="$COMMIT_SHA"
  if [ "$pubulished_commit" == "$last_commit" ]; then
    echo "No new commits, exiting, skipping all steps"
    echo "skip=true" >> "$GITHUB_OUTPUT"
    exit 0
  fi
  VERSION="$CARGO_PKG_VERSION-alpha.$(date +%s)"
  publish="true"
elif [ "$GITHUB_EVENT_NAME" == "workflow_dispatch" ]; then
  echo "Workflow dispatch event detected, reading inputs"
  beta=$(yq -r '.inputs.beta' "$GITHUB_EVENT_PATH")
  if [ "$beta" == "true" ]; then
    echo "Beta flag detected, marking version as beta pre-release and publish to beta channel"
    beta_number=$(yq -r ".details.beta_number" version/beta.json)
    VERSION="$CARGO_PKG_VERSION-beta.$beta_number"
    channel="beta"
  else
    echo "No beta flag detected, marking version as stable release and publish to stable channel"
    channel="stable"
  fi
  publish=$(yq -r '.inputs.publish' "$GITHUB_EVENT_PATH")
else
  REF_VERSION=${GITHUB_REF#refs/tags/v}
  if [ "$REF_VERSION" == "$GITHUB_REF" ]; then
    echo "Version tag not matched, aborting"
    exit 1
  fi
  echo "Tag detected, marking version as stable release and publish to stable channel"
  channel="stable"
  VERSION="$REF_VERSION"
fi
echo "Release version $VERSION to $channel channel and publish=$publish"
{
  echo "channel=$channel"
  echo "commit=$COMMIT_SHA"
  echo "version=$VERSION"
  echo "publish=$publish"
  echo "skip=false"
} >> "$GITHUB_OUTPUT"
