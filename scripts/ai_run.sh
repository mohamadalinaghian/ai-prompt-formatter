#!/usr/bin/env bash
# ==============================================================================
# pfmt: macOS Karabiner & Shell Runner
# Intercepts prompt in frontmost application, enhances it via pfmt, and pastes back.
# ==============================================================================

set -uo pipefail

export LC_ALL=en_US.UTF-8
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:$PATH"

# 1. Lockfile concurrency protection
LOCK_DIR="/tmp/pfmt.lock"
acquire_lock() {
    local now
    now=$(date +%s)
    if ! mkdir "$LOCK_DIR" 2>/dev/null; then
        if [ -f "$LOCK_DIR/pid" ]; then
            local old_pid old_time age
            old_pid=$(cat "$LOCK_DIR/pid" 2>/dev/null || echo "")
            old_time=$(cat "$LOCK_DIR/time" 2>/dev/null || echo "$now")
            age=$((now - old_time))
            if [ -n "$old_pid" ] && ! kill -0 "$old_pid" 2>/dev/null; then
                # Dead PID, clean up
                rm -rf "$LOCK_DIR"
            elif [ "$age" -gt 30 ]; then
                # Stale lock > 30s
                rm -rf "$LOCK_DIR"
            else
                echo "pfmt is already running (PID $old_pid). Exiting." >&2
                exit 0
            fi
        fi
        if ! mkdir "$LOCK_DIR" 2>/dev/null; then
            echo "Failed to acquire lock." >&2
            exit 0
        fi
    fi
    echo "$$" > "$LOCK_DIR/pid"
    echo "$now" > "$LOCK_DIR/time"
}

cleanup() {
    rm -rf "$LOCK_DIR"
}
trap cleanup EXIT INT TERM

acquire_lock

# 2. Debounce sleep to let Karabiner physical modifiers release
sleep 0.4

# 3. Detect frontmost application
FRONT_APP=$(osascript -e 'tell application "System Events" to get name of first application process whose frontmost is true' 2>/dev/null || echo "Unknown")
FRONT_APP=$(echo "$FRONT_APP" | xargs)

# 4. Locate pfmt binary
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

if command -v pfmt >/dev/null 2>&1; then
    PFMT_BIN=$(command -v pfmt)
elif [ -x "$REPO_DIR/target/release/pfmt" ]; then
    PFMT_BIN="$REPO_DIR/target/release/pfmt"
elif [ -x "$REPO_DIR/target/debug/pfmt" ]; then
    PFMT_BIN="$REPO_DIR/target/debug/pfmt"
elif [ -x "$HOME/.local/bin/pfmt" ]; then
    PFMT_BIN="$HOME/.local/bin/pfmt"
else
    osascript -e 'display notification "pfmt executable not found in PATH" with title "pfmt" sound name "Basso"'
    exit 1
fi

# 5. Backup current clipboard
PREV_CLIPBOARD="$(pbpaste 2>/dev/null || echo "")"

# 6. Copy text from frontmost application
# Clear clipboard temporarily to detect if copy succeeds
printf "" | pbcopy

# Synthesize Cmd+A then Cmd+C
osascript 2>/dev/null <<EOF
tell application "System Events"
    keystroke "a" using command down
    delay 0.05
    keystroke "c" using command down
end tell
EOF

# Wait up to 500ms for clipboard content
COPIED=""
for _ in $(seq 1 10); do
    COPIED="$(pbpaste 2>/dev/null || echo "")"
    if [ -n "$COPIED" ]; then
        break
    fi
    sleep 0.05
done

# If nothing was copied, user has nothing selected/typed
if [ -z "$COPIED" ]; then
    # Restore original clipboard
    printf "%s" "$PREV_CLIPBOARD" | pbcopy
    osascript -e 'display notification "Empty prompt: nothing to enhance." with title "pfmt"'
    exit 0
fi

# 7. Execute pfmt enhancement pipeline
STDERR_FILE=$(mktemp /tmp/pfmt_stderr.XXXXXX)
ENHANCED_OUTPUT=$(printf "%s" "$COPIED" | pfmt-cli enhance --app "$FRONT_APP" --no-llm 2> "$STDERR_FILE")
EXIT_CODE=$?
STATUS_MSG=$(head -n 1 "$STDERR_FILE" | tr -d '\r' || echo "Completed")
rm -f "$STDERR_FILE"

# Escape double quotes in status message for osascript
ESCAPED_MSG=$(echo "$STATUS_MSG" | sed 's/"/\\"/g')

# 8. Handle degradation ladder and pasting
if [ "$EXIT_CODE" -eq 0 ]; then
    # Success / Idempotent unchanged
    if [ "$STATUS_MSG" = "unchanged" ]; then
        # Already formatted, restore original text to clipboard and notify
        printf "%s" "$COPIED" | pbcopy
        osascript -e "display notification \"Prompt already formatted\" with title \"pfmt\" subtitle \"Unchanged\" sound name \"Glass\""
    else
        # Paste enhanced output
        printf "%s" "$ENHANCED_OUTPUT" | pbcopy
        sleep 0.05
        osascript 2>/dev/null <<EOF
tell application "System Events"
    keystroke "v" using command down
end tell
EOF
        osascript -e "display notification \"$ESCAPED_MSG\" with title \"pfmt\" subtitle \"Enhanced for $FRONT_APP\" sound name \"Glass\""
    fi
elif [ "$EXIT_CODE" -eq 10 ]; then
    # Degraded output (paste with banner)
    printf "%s" "$ENHANCED_OUTPUT" | pbcopy
    sleep 0.05
    osascript 2>/dev/null <<EOF
tell application "System Events"
    keystroke "v" using command down
end tell
EOF
    osascript -e "display notification \"⚠️ $ESCAPED_MSG\" with title \"pfmt\" subtitle \"Degraded Output\" sound name \"Basso\""
else
    # Abort (exit code 1 or other) - DO NOT PASTE! Original text remains untouched.
    printf "%s" "$PREV_CLIPBOARD" | pbcopy
    osascript -e "display notification \"❌ $ESCAPED_MSG\" with title \"pfmt\" subtitle \"Aborted (original untouched)\" sound name \"Basso\""
fi
