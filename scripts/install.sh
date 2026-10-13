#!/usr/bin/env bash
# ==============================================================================
# pfmt: macOS Installer
# Builds release binary, installs CLI and wrapper to ~/.local/bin,
# configures LaunchAgent daemon for pfmt watch, and installs Karabiner rule.
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "🚀 Installing pfmt..."

# 1. Build release binary
echo "📦 Building pfmt release binary..."
cd "$REPO_DIR"
cargo build --release

# 2. Setup ~/.local/bin and install pfmt
INSTALL_BIN_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_BIN_DIR"
mkdir -p "$HOME/.config/pfmt"

echo "📂 Installing executable to $INSTALL_BIN_DIR/pfmt..."
# NOTE: remove before copy — overwriting an ad-hoc-signed binary in place
# leaves macOS with a stale code-signature cache and the new binary gets
# SIGKILLed ("Taskgated Invalid Signature"). A fresh inode avoids that.
rm -f "$INSTALL_BIN_DIR/pfmt"
cp "$REPO_DIR/target/release/pfmt" "$INSTALL_BIN_DIR/pfmt"
chmod +x "$INSTALL_BIN_DIR/pfmt"

echo "📂 Installing runner script to $INSTALL_BIN_DIR/ai_run.sh..."
cp "$REPO_DIR/scripts/ai_run.sh" "$INSTALL_BIN_DIR/ai_run.sh"
chmod +x "$INSTALL_BIN_DIR/ai_run.sh"

# 3. Setup default config & profile if not already present in ~/.config/pfmt
if [ ! -f "$HOME/.config/pfmt/config.toml" ]; then
    if [ -f "$REPO_DIR/config.toml" ]; then
        echo "⚙️  Copying default config.toml to ~/.config/pfmt/..."
        cp "$REPO_DIR/config.toml" "$HOME/.config/pfmt/config.toml"
    fi
fi

if [ ! -f "$HOME/.config/pfmt/profiles/default.toml" ]; then
    mkdir -p "$HOME/.config/pfmt/profiles"
    if [ -f "$REPO_DIR/profiles/default.toml" ]; then
        echo "⚙️  Copying default profile to ~/.config/pfmt/profiles/..."
        cp "$REPO_DIR/profiles/default.toml" "$HOME/.config/pfmt/profiles/default.toml"
    fi
fi

# 4. Install Karabiner-Elements complex modification rule
KARABINER_DIR="$HOME/.config/karabiner/assets/complex_modifications"
if [ -d "$HOME/.config/karabiner" ]; then
    mkdir -p "$KARABINER_DIR"
    echo "⌨️  Installing Karabiner rule to $KARABINER_DIR/pfmt.json..."
    cp "$REPO_DIR/scripts/karabiner/pfmt.json" "$KARABINER_DIR/pfmt.json"
    echo "ℹ️  To enable the shortcut in Karabiner-Elements:"
    echo "    Open Karabiner-Elements -> Complex Modifications -> Add Rule -> Enable 'Enhance prompt in frontmost agent with pfmt'"
fi

# 5. Setup LaunchAgent daemon for pfmt watch
LAUNCH_AGENTS_DIR="$HOME/Library/LaunchAgents"
mkdir -p "$LAUNCH_AGENTS_DIR"
PLIST_TARGET="$LAUNCH_AGENTS_DIR/com.pfmt.watch.plist"

echo "🕒 Configuring background watcher daemon at $PLIST_TARGET..."
sed "s|__HOME__|$HOME|g" "$REPO_DIR/scripts/launchd/pfmt-watch.plist" > "$PLIST_TARGET"

# Reload launchd service
echo "🔄 Starting pfmt watch daemon via launchctl..."
launchctl unload "$PLIST_TARGET" 2>/dev/null || true
launchctl load -w "$PLIST_TARGET" 2>/dev/null || true

# 6. Run doctor verification
echo ""
echo "🩺 Running pfmt doctor verification..."
"$INSTALL_BIN_DIR/pfmt" doctor

echo ""
echo "✨ pfmt installation complete!"
echo "   Binary: $INSTALL_BIN_DIR/pfmt"
echo "   Runner: $INSTALL_BIN_DIR/ai_run.sh"
echo "   Watcher: com.pfmt.watch (running via launchd)"
echo "   Shortcut: Cmd + Shift + Space (via Karabiner — actual pfmt.json binding; AGENTS.md mentions Ctrl+Opt+P as reference only)"
