#!/bin/bash

set -e

CARGO_PKG_VERSION=$(yq -oy '.package.version' maa-cli/Cargo.toml)
COMMIT_SHA=$(git rev-parse HEAD)
COMMIT_SHORT_SHA=$(git rev-parse --short HEAD)

# Semantic version parse
#
# For a stable version, the version number should be in the format of `x.y.z`
# For a beta version, the version number should be in the format of `x.y.z-beta.n`
# For an alpha version, the version number should be in the format of
# `x.y.z-alpha.n+sha.abcdef` or `x.y.z-beta.n.alpha.m+sha.abcdef`
#
# there are three parts of a semantic version:
# 1. core: `x.y.z`
# 2. pre-release: `alpha.n` or `beta.n.alpha.m`
# 3. build metadata: `sha.abcdef` (in this case, it's a short commit sha)
parse_semver() {
  pre_build_metadata=${1%+*}
  build_metadata=${1#*+}

  if [[ "$pre_build_metadata" == "$build_metadata" ]]; then
    build_metadata=""
  fi

  core=${pre_build_metadata%-*}
  pre_release=${pre_build_metadata#*-}

  if [[ "$core" == "$pre_release" ]]; then
    pre_release=""
  fi
}

# Check if the version in Cargo.toml is bumped
# If the version in Cargo.toml is the same as the published version,
# then no pre-release is allowed for the same version.
#
# For example, if the published version and the version in Cargo.toml is `0.4.5`,
# then an alpha version will be `0.4.5-alpha.1+sha.abcdef` which is less than `0.4.5`
check_version_bumped() {
  local published_version=$1
  if [[ "$CARGO_PKG_VERSION" == "$published_version" ]]; then
    echo "The version in Cargo.toml is the same as the published version"
    echo "No pre-release is allowed for the same version"
    echo "skip=true" >> "$GITHUB_OUTPUT"
    exit 0
  fi
}

# determine the channel and whether to publish
if [[ "$GITHUB_EVENT_NAME" == "pull_request" ]]; then
  echo "PR detected"
  channel="alpha"
  publish="false"
elif [[ "$GITHUB_EVENT_NAME" == "schedule" ]]; then
  echo "Scheduled event detected"
  channel="alpha"
  publish="true"
elif [[ "$GITHUB_EVENT_NAME" == "workflow_dispatch" ]]; then
  echo "Workflow dispatch event detected"
  channel=$(yq -oy '.inputs.channel' "$GITHUB_EVENT_PATH")
  publish=$(yq -oy -r '.inputs.publish' "$GITHUB_EVENT_PATH")
elif [[ "$GITHUB_EVENT_NAME" == "push" ]]; then
  ref_version=${GITHUB_REF#refs/tags/v}
  if [[ "$ref_version" != "$CARGO_PKG_VERSION" ]]; then
    echo "Version tag not matched, aborting"
    exit 1
  fi
  echo "New tag detected"
  channel="stable"
  publish="true"
else
  echo "Unknown event $GITHUB_EVENT_NAME, aborting"
  exit 1
fi


if [[ "$channel" == "stable" ]]; then
  echo "Creating stable release"
else
  echo "Creating $channel pre-release"
fi

if [[ $publish == "true" ]]; then
  echo "Publishing to $channel channel"
else
  echo "Skipping publish"
fi

# skip if no new commits
published_commit=$(yq -oy ".details.commit" "version/$channel.json")
if [[ "$published_commit" == "$COMMIT_SHA" ]]; then
  echo "No new commits, exiting, skipping all steps"
  echo "skip=true" >> "$GITHUB_OUTPUT"
  exit 0
fi

published_version=$(yq -oy ".version" "version/$channel.json")
if [[ "$channel" == "beta" ]]; then
  check_version_bumped "$published_version"
  parse_semver "$published_version"
  if [[ "$CARGO_PKG_VERSION" == "$core" ]]; then
    if [[ -z "$pre_release" ]]; then
      echo "$core-beta.1"
    else
      head=${pre_release%%.*}
      beta_number=${pre_release#*.}
      version="$core-beta.$((beta_number + 1))"
    fi
  else
    version="$CARGO_PKG_VERSION-beta.1"
  fi
  tag="v$version"
elif [[ "$channel" == "alpha" ]]; then
  check_version_bumped "$published_version"
  parse_semver "$published_version"
  if [[ "$CARGO_PKG_VERSION" == "$core" ]]; then
    if [[ -z $pre_release ]]; then
      version="$core-alpha.1+sha.$COMMIT_SHORT_SHA"
    else
      head=${pre_release%%.*}
      if [[ $head == "beta" ]]; then
        beta_rest=${pre_release#*.}
        beta_number=${beta_rest%%.*}
        alpha_rest=${beta_rest#*.}
        if [[ $beta_number == "$alpha_rest" ]]; then
          version="$core-beta.$beta_number-alpha.1+sha.$COMMIT_SHORT_SHA"
        else
          alpha_number=${alpha_rest#*.}
          version="$core-beta.$beta_number-alpha.$((alpha_number + 1))+sha.$COMMIT_SHORT_SHA"
        fi
      elif [[ $head == "alpha" ]]; then
        alpha_number=${pre_release#*.}
        version="$core-alpha.$((alpha_number + 1))+sha.$COMMIT_SHORT_SHA"
      else
        echo "Invalid pre-release version: $pre_release" >&2
        exit 1
      fi
    fi
  else
    version="$CARGO_PKG_VERSION-alpha.1+sha.$COMMIT_SHORT_SHA"
  fi
  tag="nightly"
else
  version="$CARGO_PKG_VERSION"
  tag="v$version"
fi

echo "Release version $version with tag $tag to channel $channel (publish: $publish)"
{
  echo "commit=$COMMIT_SHA"
  echo "channel=$channel"
  echo "version=$version"
  echo "tag=$tag"
  echo "publish=$publish"
  echo "skip=false"
} >> "$GITHUB_OUTPUT"
