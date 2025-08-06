#!/usr/bin/env bash

set -e

LAST_TAG=$(git describe --tags --abbrev=0 --match "v*")

# Get package name and manifest_path for all members
PACKAGES=$(cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | "\(.name):::\(.manifest_path)"')

for PKG in $PACKAGES; do
    NAME="${PKG%%:::*}"
    MANIFEST_PATH="${PKG##*:::}"
    DIR=$(dirname "$MANIFEST_PATH")

    # Look for release commit for this member up to the last tag
    RELEASE_COMMIT=$(git log --oneline --grep="^$NAME-v" --format="%H" "$LAST_TAG"..HEAD | head -n1)

    if [[ -z "$RELEASE_COMMIT" ]]; then
        # No release commit found, use the latest release tag commit
        RELEASE_COMMIT=$(git rev-list -n 1 "$LAST_TAG")
    fi

    # Check if any file in the package directory changed since the member's release commit or latest tag release
    if git diff --quiet "$RELEASE_COMMIT"..HEAD -- "$DIR"; then
        continue
    fi

    echo "Changes detected in $NAME ($DIR)"
    PS3="Select version bump for $NAME: "
    select BUMP in patch minor major; do
        if [[ "$BUMP" == "patch" || "$BUMP" == "minor" || "$BUMP" == "major" ]]; then
            break
        else
            echo "Invalid option"
        fi
    done

    if cargo set-version --help >/dev/null 2>&1; then
        cargo set-version --bump "$BUMP" --package "$NAME"
    else
        echo "Please install cargo-edit first."
        exit 1
    fi

    echo "$NAME bumped to $BUMP"
done
