#!/bin/bash

set -e

CARGO_PKG_VERSION=$(yq -oy -r '.package.version' maa-cli/Cargo.toml)
commit_sha=$(git rev-parse HEAD)

release_alpha() {
  channel="alpha"
  published_commit=$(yq -oy -r ".details.commit" version/$channel.json)
  if [ "$published_commit" == "$commit_sha" ]; then
    echo "No new commits, exiting, skipping all steps"
    echo "skip=true" >> "$GITHUB_OUTPUT"
    exit 0
  fi
  version="$CARGO_PKG_VERSION-alpha.$(date +%s)"
  tag="nightly"
}

release_beta() {
  channel="beta"
  published_version=$(yq -oy -r ".details.version" version/$channel.json)
  published_version_prefix=${published_version%-*}
  published_version_suffix=${published_version#*-}
  if [ "$published_version_prefix" != "$CARGO_PKG_VERSION" ]; then
    echo "Last published version is not the same as current version (published: $published_version)"
    version="$CARGO_PKG_VERSION-$channel.1"
  elif [ "$published_version_suffix" == "$published_version" ]; then
    echo "Last published version is not a pre-release version (published: $published_version)"
    version="$CARGO_PKG_VERSION-$channel.1"
  else
    beta_number=${published_version_suffix#*.}
    version="$CARGO_PKG_VERSION-$channel.$((beta_number + 1))"
  fi
}

if [ "$GITHUB_EVENT_NAME" == "pull_request" ]; then
  echo "PR detected, marking version as alpha pre-release and skipping publish"
  release_alpha
  publish="false"
elif [ "$GITHUB_EVENT_NAME" == "schedule" ]; then
  echo "Scheduled event detected, marking version as alpha pre-release and publish to alpha channel"
  release_alpha
  tag="nightly"
elif [ "$GITHUB_EVENT_NAME" == "workflow_dispatch" ]; then
  echo "Workflow dispatch event detected, reading inputs"
  channel=$(yq -oy -r '.inputs.channel' "$GITHUB_EVENT_PATH")
  if [ "$channel" == "alpha" ]; then
    echo "Dispatched alpha channel, marking version as alpha pre-release"
    release_alpha
    tag="nightly"
  elif [ "$channel" == "beta" ]; then
    echo "Beta channel detected, marking version as beta pre-release and publish to beta channel"
    release_beta
  elif [ "$channel" == "stable" ]; then
    echo "Stable channel detected, marking version as stable release and publish to stable channel"
  else
    echo "Unknown channel $channel, aborting"
    exit 1
  fi
  publish=$(yq -oy -r '.inputs.publish' "$GITHUB_EVENT_PATH")
elif [ "$GITHUB_EVENT_NAME" == "push" ]; then
  ref_version=${GITHUB_REF#refs/tags/v}
  if [ "$ref_version" != "$CARGO_PKG_VERSION" ]; then
    echo "Version tag not matched, aborting"
    exit 1
  fi
  echo "Tag detected, marking version as stable release and publish to stable channel"
else
  echo "Unknown event $GITHUB_EVENT_NAME, aborting"
  exit 1
fi

channel=${channel:-stable}
version=${version:-$CARGO_PKG_VERSION}
tag=${tag:-v$version}
publish=${publish:-true}

if [ "$channel" != "beta" ]; then
  # both stable and alpha channel are compared to the last stable release
  compare_base=$(yq -oy -r ".details.commit" version/stable.json)
else
  # beta channel is compared to the last beta release
  compare_base=$(yq -oy -r ".details.commit" version/beta.json)
fi

echo "Release version $version with tag $tag to channel $channel (publish: $publish)"
{
  echo "commit=$commit_sha"
  echo "channel=$channel"
  echo "version=$version"
  echo "tag=$tag"
  echo "compare_base=$compare_base"
  echo "publish=$publish"
  echo "skip=false"
} >> "$GITHUB_OUTPUT"
