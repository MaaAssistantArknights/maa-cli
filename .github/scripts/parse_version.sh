#!/bin/bash

set -e

CARGO_PKG_VERSION=$(yq -r '.package.version' maa-cli/Cargo.toml)

if [ "$GITHUB_EVENT_NAME" == "pull_request" ]; then
  echo "PR detected, marking version as alpha pre-release and skipping publish"
  VERSION="$CARGO_PKG_VERSION-alpha.$(date +%s)"
  echo "channel=alpha" > $GITHUB_OUTPUT
  echo "publish=false" > $GITHUB_OUTPUT
elif [ "$GITHUB_EVENT_NAME" == "schedule" ]; then
  echo "Scheduled event detected, marking version as alpha pre-release and publish to alpha channel"
  # check if there are some new commits
  channel="alpha"
  pubulished_commit=$(yq -r ".details.commit" version/$channel.json)
  last_commit=$(git rev-parse HEAD)
  if [ "$pubulished_commit" == "$last_commit" ]; then
    echo "No new commits, exiting, skipping all steps"
    echo "skip=true" > $GITHUB_OUTPUT
    exit 0
  fi
  VERSION="$CARGO_PKG_VERSION-alpha.$(date +%s)"
  echo "channel=$channel" > $GITHUB_OUTPUT
  echo "publish=true" > $GITHUB_OUTPUT
elif [ "$GITHUB_EVENT_NAME" == "workflow_dispatch" ]; then
  echo "Workflow dispatch event detected, reading inputs"
  beta=$(yq -r '.inputs.beta' $GITHUB_EVENT_PATH)
  if [ "$beta" == "true" ]; then
    echo "Beta flag detected, marking version as beta pre-release and publish to beta channel"
    beta_number=$(yq -r ".details.beta_number" version/beta.json)
    VERSION="$CARGO_PKG_VERSION-beta.$beta_number"
    channel="beta"
  else
    echo "No beta flag detected, marking version as stable release and publish to stable channel"
    channel="stable"
  fi
  publish=$(yq -r '.inputs.publish' $GITHUB_EVENT_PATH)
  echo "channel=$channel" > $GITHUB_OUTPUT
  echo "publish=$publish" > $GITHUB_OUTPUT
else
  REF_VERSION=${GITHUB_REF#refs/tags/v}
  if [ "$REF_VERSION" == "$GITHUB_REF" ]; then
    echo "Current commit is not tagged, skipping all steps"
    echo "skip_all=true" > $GITHUB_OUTPUT
    exit 0
  fi
  echo "Tag detected, marking version as stable release and publish to stable channel"
  channel="stable"
  VERSION="$REF_VERSION"
fi
echo "Release version: $VERSION"
echo "version=$VERSION" > $GITHUB_OUTPUT
