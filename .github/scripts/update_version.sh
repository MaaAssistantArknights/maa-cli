#!/bin/bash

set -e

VERSION=$1
CHANNEL=$2

targets=(
  x86_64-unknown-linux-gnu
  aarch64-unknown-linux-gnu
  universal-apple-darwin
  x86_64-pc-windows-msvc
)

version_files=(
  version/alpha.json
)
[ "$CHANNEL" != "alpha" ] && version_files+=(version/beta.json)
[ "$CHANNEL" == "stable" ] && version_files+=(version/stable.json)

# target independent version info
for version_file in "${version_files[@]}"; do
  yq -i -oj ".version = \"$VERSION\"" $version_file
  yq -i -oj ".details.tag = \"v$VERSION\"" $version_file
  yq -i -oj ".details.commit = \"$(git rev-parse HEAD)\"" $version_file
done

for target in "${targets[@]}"; do
  dir="maa_cli-$target"
  tar -xvf $dir.tar -C $target_dir
  # use tar on linux and zip on other platforms
  if [[ "$target" == *"linux"* ]]; then
    archive_name="maa_cli-v$version-$target.tar.gz"
    tar -czvf $archive_name $dir/maa
  else
    archive_name="maa_cli-v$version-$target.zip"
    zip -r $archive_name $dir/maa
  fi
  checksum=$(sha256sum $archive_name)
  checksum_hash=${checksum:0:64}
  size=$(stat -c%s $archive_name)
  echo $checksum > $archive_name.sha256

  # old version info (deprecated)
  version_file="version/version.json"
  yq -i -oj ".maa-cli.$target.version = \"$version\"" $version_file
  yq -i -oj ".maa-cli.$target.tag = \"v$version\"" $version_file
  yq -i -oj ".maa-cli.$target.name = \"$archive_name\"" $version_file
  yq -i -oj ".maa-cli.$target.size = $size" $version_file
  yq -i -oj ".maa-cli.$target.sha256sum = \"$checksum_hash\"" $version_file

  # target dependent version info
  for version_file in "${version_files[@]}"; do
    yq -i -oj ".details.assets.$target.name = \"$archive_name\"" $version_file
    yq -i -oj ".details.assets.$target.size = $size" $version_file
    yq -i -oj ".details.assets.$target.sha256sum = \"$checksum_hash\"" $version_file
  done
done
