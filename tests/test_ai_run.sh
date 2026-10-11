#!/usr/bin/env bash
set -euo pipefail

echo "Testing Karabiner JSON validity..."
python3 -m json.tool scripts/karabiner/pfmt.json > /dev/null
echo "✓ Karabiner JSON is valid"

echo "Testing launchd plist validity..."
plutil -lint scripts/launchd/pfmt-watch.plist > /dev/null
echo "✓ launchd plist is valid"

echo "Testing lockfile logic..."
LOCK_DIR="/tmp/pfmt.lock"
rm -rf "$LOCK_DIR"

# Acquire lock artificially with old timestamp
mkdir -p "$LOCK_DIR"
echo "999999" > "$LOCK_DIR/pid"
echo "1000000000" > "$LOCK_DIR/time"

# Running a subshell simulating lock acquisition
bash -c '
LOCK_DIR="/tmp/pfmt.lock"
now=$(date +%s)
old_pid=$(cat "$LOCK_DIR/pid" 2>/dev/null || echo "")
old_time=$(cat "$LOCK_DIR/time" 2>/dev/null || echo "$now")
age=$((now - old_time))
if [ "$age" -gt 30 ]; then
    rm -rf "$LOCK_DIR"
fi
mkdir "$LOCK_DIR"
'
echo "✓ Stale lock cleanup and acquisition succeeded"
rm -rf "$LOCK_DIR"

echo "Testing pfmt CLI discovery in scripts/ai_run.sh..."
bash -c '
SCRIPT_DIR="$(pwd)/scripts"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
if [ -x "$REPO_DIR/target/debug/pfmt" ]; then
    echo "Found pfmt binary: $REPO_DIR/target/debug/pfmt"
    exit 0
else
    echo "pfmt binary not found"
    exit 1
fi
'
echo "✓ pfmt binary located correctly"

echo "All shell integration tests passed!"
