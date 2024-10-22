#!/bin/sh
INPUT_DIR="public/js"
OUTPUT_FILE="${INPUT_DIR}/bundle.js"

rm "$OUTPUT_FILE"

find "$INPUT_DIR" -name "*.js" ! -name "bundle.js" | while read -r file; do
    printf "\n// %s \n\n" "$file" >> "$OUTPUT_FILE"
    cat "$file" >> "$OUTPUT_FILE"
done