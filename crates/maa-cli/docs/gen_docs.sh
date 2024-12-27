#!/bin/bash
# generate documentation used in main repo of MAA
# usage: ./gen_docs.sh <lang> [<output_dir>]

resolve_dir() {
  old_dir=$(pwd)
  cd -- "$1" &> /dev/null && pwd
  cd -- "$old_dir" &> /dev/null
}

files=(
  install.md
  usage.md
  config.md
  faq.md
)

icons=(
  material-symbols:download
  material-symbols:summarize
  material-symbols:settings
  ph:question-fill
)

sedi() {
  if [[ "$OSTYPE" == "darwin"* ]]; then
    sed -i '' "$@"
  else
    sed -i "$@"
  fi
}

# language of the documentation
for lang in en-US zh-CN zh-TW ja-JP ko-KR; do
  lang_lower=$(echo "$lang" | tr '[:upper:]' '[:lower:]')
  # output to the specified directory, default is the same as the language
  output_dir=$(resolve_dir "$lang_lower/manual/cli")
  # the original directory of docs is at the same directory as this script
  original_dir=$(resolve_dir "$(dirname "${BASH_SOURCE[0]}")")

  echo "Generating documentation for $lang"
  order=0
  for filename in "${files[@]}"; do
    echo "-> Generating documentation for $filename"
    file="$original_dir/$lang/$filename"
    index=$order
    order=$((order+1))
    out_file="$output_dir/$filename"
    # insert metadata of markdown file to the beginning of the file
    {
      echo "---"
      echo "order: $order"
      echo "icon: ${icons[$index]}"
      echo "---"
      echo
      cat "$file"
    } > "$out_file"
    # remap some relative links to github links
    sedi -E 's|\.\./\.\./|https://github.com/MaaAssistantArknights/maa-cli/blob/main/maa-cli/|g' "$out_file"
    # remap maa docs links to the relative links
    sedi -E 's|https://maa\.plus/docs/[^/]+/(.+)\.html|../../\1.md|g' "$out_file"
  done
done
