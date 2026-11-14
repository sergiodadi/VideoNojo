#!/usr/bin/env bash
# Refreshes testdata/ (a read-only copy, for vc_export) from the safe
# working copy of the real collection — never from the live file at
# ~/Documents/Collectorz/Movie Collector/... directly.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOURCE="$HOME/Documents/Temp/Movie/MyCollection.mvcp"
DEST="$SCRIPT_DIR/testdata/MyCollection.mvcp"

if [ ! -d "$SOURCE" ]; then
  echo "Source not found: $SOURCE" >&2
  exit 1
fi

rm -rf "${SCRIPT_DIR:?}/testdata"
mkdir -p "$SCRIPT_DIR/testdata"
cp -R "$SOURCE" "$DEST"
chmod -R a-w "$DEST"

echo "OK: testdata/MyCollection.mvcp refreshed (read-only)."
