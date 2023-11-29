#!/bin/bash

set -e

CARGO_PKG_VERSION=$(yq -r '.package.version' maa-cli/Cargo.toml)
commit_sha=$(git rev-parse HEAD)

if [ "$GITHUB_EVENT_NAME" == "pull_request" ]; then
  echo "PR detected, marking version as alpha pre-release and skipping publish"
  channel="alpha"
  version="$CARGO_PKG_VERSION-alpha.$(date +%s)"
  tag="nightly"
  publish="false"
elif [ "$GITHUB_EVENT_NAME" == "schedule" ]; then
  echo "Scheduled event detected, marking version as alpha pre-release and publish to alpha channel"
  # check if there are some new commits
  channel="alpha"
  published_commit=$(yq -r ".details.commit" version/$channel.json)
  if [ "$published_commit" == "$commit_sha" ]; then
    echo "No new commits, exiting, skipping all steps"
    echo "skip=true" >> "$GITHUB_OUTPUT"
    exit 0
  fi
  version="$CARGO_PKG_VERSION-alpha.$(date +%s)"
  tag="nightly"
elif [ "$GITHUB_EVENT_NAME" == "workflow_dispatch" ]; then
  echo "Workflow dispatch event detected, reading inputs"
  beta=$(yq -r '.inputs.beta' "$GITHUB_EVENT_PATH")
  if [ "$beta" == "true" ]; then
    echo "Beta flag detected, marking version as beta pre-release and publish to beta channel"
    channel="beta"
    published_version=$(yq -r ".details.version" version/beta.json)
    published_version_prefix=${published_version%-*}
    published_version_suffix=${published_version#*-}
    if [ "$published_version_prefix" != "$CARGO_PKG_VERSION" ]; then
      echo "Version prefix not matched (published: $published_version_prefix, expected: $CARGO_PKG_VERSION)"
      exit 1
    elif [ "$published_version_suffix" == "$published_version" ]; then
      echo "Last published version is not a pre-release version (published: $published_version)"
      version="$CARGO_PKG_VERSION-$channel.1"
    else
      beta_number=${published_version_suffix#*.}
      version="$CARGO_PKG_VERSION-$channel.$((beta_number + 1))"
    fi
  else
    echo "No beta flag detected, marking version as stable release and publish to stable channel"
    channel="stable"
  fi
  publish=$(yq -r '.inputs.publish' "$GITHUB_EVENT_PATH")
else # push to a tag
  REF_VERSION=${GITHUB_REF#refs/tags/v}
  if [ "$REF_VERSION" == "$GITHUB_REF" ]; then
    echo "Version tag not matched, aborting"
    exit 1
  fi
  echo "Tag detected, marking version as stable release and publish to stable channel"
  channel="stable"
  version="$REF_VERSION"
fi

echo "Release version $version to $channel channel with tag $tag"
{
  echo "commit=$commit_sha"
  echo "channel=$channel"
  echo "version=$version"
  echo "tag=${tag:-v$version}"
  echo "publish=${publish:-ture}"
  echo "skip=false"
} >> "$GITHUB_OUTPUT"
