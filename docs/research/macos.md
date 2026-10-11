# macOS Integration Research

Access Date: 2026-10-11
Sources:
- Karabiner-Elements complex modifications documentation
- AppleScript System Events reference
- Apple Keychain Services API & `security` CLI
- launchd.plist(5) man page

## 1. Karabiner Hotkey Flow
- Complex modification triggers `shell_command` with clean environment.
- Must source `/etc/profile` and `~/.profile` / `~/.zshrc` or resolve paths explicitly.
- Shortcut: e.g. `Cmd+Shift+Space`.
- Timing: `sleep 0.4` for physical modifier release before issuing synthetic keystrokes.

## 2. Text Box Selection (Terminal vs GUI)
- Terminal apps (`stable`, `Warp`, `Terminal`, `iTerm2`, `Ghostty`, `kitty`, `alacritty`): `Cmd+A` selects entire scrollback buffer. Must use `Ctrl+A` then `Ctrl+C` (or `Ctrl+U`/`Ctrl+K` depending on shell bindings).
- GUI apps (`Antigravity`, `Freebuff`, `OpenCode`, etc.): Standard `Cmd+A` then `Cmd+C`.
- Delay: 0.2s after copy/paste for pasteboard synchronization.

## 3. Frontmost App Detection
- AppleScript query:
  `tell application "System Events" to get name of first application process whose frontmost is true`
- Returns app name (e.g. `Antigravity`, `OpenCode`, `Ghostty`).

## 4. Keychain Access
- CLI command: `security find-generic-password -s <service_name> -w`
- Rust crate: `keyring` (v2.x or 3.x) wraps `SecItemCopyMatching`.
- Fallback: Environment variable (`GROQ_API_KEY`, `TYPESAFE_API_KEY`) when Keychain entry is absent.

## 5. File Watching & launchd
- SQLite WAL files: Write transactions update `<name>.sqlite-wal` and `<name>.sqlite-shm`. File watcher must monitor parent directory or both base and `-wal` files.
- LaunchAgent: Located at `~/Library/LaunchAgents/com.user.pfmt-watch.plist`.
