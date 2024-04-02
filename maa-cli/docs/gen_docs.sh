#!/bin/bash
# generate documentation used in main repo of MAA

# output to the current directory
output_dir=$1
# the original directory of docs is at the same directory as this script
original_dir=$(cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd)

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
  for file in "$lang"/{intro,install,usage,config,faq}.md; do
    echo "-> Generating documentation for $file"

    index=$order
    order=$((order+1))
    out_file="$output_dir/cli-$(basename "$file")"
    # insert metadata of markdown file to the beginning of the file
    {
      echo "---"
      echo "order: $order"
      echo "icon: ${icons[$index]}"
      echo "---"
      echo
    } > "$out_file"
    # remap some links to the original repo
    sed 's|\.\./\.\./|https://github.com/MaaAssistantArknights/maa-cli/blob/main/maa-cli/|g' < "$file" |
      sed 's|https://maa\.plus/docs/|../../|g' >> "$out_file"

    echo "-> Done"
  done
done
