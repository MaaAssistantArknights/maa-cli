#!/bin/bash
# generate documentation used in main repo of MAA

# output to the specified directory
output_dir=$1
# the original directory of docs is at the same directory as this script
original_dir=$(cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd)


files=(
  intro.md
  install.md
  usage.md
  config.md
  faq.md
)

icons=(
  material-symbols:summarize
  material-symbols:download
  material-symbols:format_list_bulleted
  material-symbols:settings
  ph:question-fill
)

# walk through all subdirectories of each language
for lang in "$original_dir"/zh-CN; do
  echo "Generating documentation for $lang"
  order=0
  for filename in "${files[@]}"; do
    echo "-> Generating documentation for $filename"
    file="$lang/$filename"
    index=$order
    order=$((order+1))
    out_file="$output_dir/cli-$filename"
    # insert metadata of markdown file to the beginning of the file
    {
      echo "---"
      echo "order: $order"
      echo "icon: ${icons[$index]}"
      echo "---"
      echo
      cat "$file"
    } > "$out_file"
    # remap some links to the original repo
    sed -I '' -E 's|\.\./\.\./|https://github.com/MaaAssistantArknights/maa-cli/blob/main/maa-cli/|g' "$out_file"
    sed -I '' -E 's|https://maa\.plus/docs/(.+)\.html|../../\1.md|g' "$out_file"
    for filename_md in "${files[@]}"; do
      sed -i '' -E "s|$filename_md|cli-$filename_md|g" "$out_file"
    done
  done
done
