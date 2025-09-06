#!/bin/sh
# This script updates the version number for
# the addon based on the current commit timestamp

set -eu

TAG_NAME=${TAG_NAME:-$(git -c "core.abbrev=8" show -s "--format=%cd-%h" "--date=format:%Y.%m.%d")}

CONFIG_FILE="addon/config.yaml"

# Use a portable in-place sed invocation (BSD vs GNU)
case "$(uname -s)" in
  Darwin*)
    sed -i "" -e "s/^version:.*/version: \"$TAG_NAME\"/" "$CONFIG_FILE"
    ;;
  *)
    sed -i -e "s/^version:.*/version: \"$TAG_NAME\"/" "$CONFIG_FILE"
    ;;
esac
