#!/usr/bin/env bash

set -e

exit_error() {
    echo "ERROR: $1" >&2
    exit 1
}

current_dir=$(pwd)

if [[ "$current_dir" != */leptos ]]; then
    exit_error "Current directory does not end with leptos"
fi

# Check if a date is provided as an argument
if [ $# -eq 1 ]; then
    NEW_DATE=$1
    echo -n "Will use the provided date: "
else
    # Use current date if no date is provided
    NEW_DATE=$(date +"%Y-%m-%d")
    echo -n "Will use the current date: "
fi

echo "$NEW_DATE"

# Detect if it is darwin sed
if sed --version 2>/dev/null | grep -q GNU; then
    SED_COMMAND="sed -i"
else
    SED_COMMAND='sed -i ""'
fi

MATCH="nightly-[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]"
REPLACE="nightly-$NEW_DATE"

# Find all occurrences of the pattern
git grep -l "$MATCH" | while read -r file; do
    # Replace the pattern in each file
    $SED_COMMAND "s/$MATCH/$REPLACE/g" "$file"
    echo "Updated $file"
done

echo "All occurrences of 'nightly-XXXX-XX-XX' have been replaced with 'nightly-$NEW_DATE'"
