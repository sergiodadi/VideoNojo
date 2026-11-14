#!/usr/bin/env bash
# Refreshes testdata-rw/ (a writable copy, for vc_add) from the safe
# working copy of the real collection — never from the live file.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOURCE="$HOME/Documents/Temp/Movie/MyCollection.mvcp"
DEST="$SCRIPT_DIR/testdata-rw/MyCollection.mvcp"

if [ ! -d "$SOURCE" ]; then
  echo "Source not found: $SOURCE" >&2
  exit 1
fi

rm -rf "${SCRIPT_DIR:?}/testdata-rw"
mkdir -p "$SCRIPT_DIR/testdata-rw"
cp -R "$SOURCE" "$DEST"
chmod -R u+w "$DEST"

echo "OK: testdata-rw/MyCollection.mvcp refreshed (writable)."
