# Research: macOS launchd Agents

- **Source URL:** macOS `man launchd.plist`, Apple Developer Library
- **Access Date:** 2026-10-11
- **Target:** macOS LaunchAgents (`~/Library/LaunchAgents/`)

## Findings

1. **User LaunchAgents:**
   - Per-user background services live in `~/Library/LaunchAgents/com.pfmt.watch.plist`.
   - Loaded and started via `launchctl bootstrap gui/$UID ~/Library/LaunchAgents/com.pfmt.watch.plist` or legacy `launchctl load -w ~/Library/LaunchAgents/com.pfmt.watch.plist`.

2. **KeepAlive and Restart Semantics:**
   - `<key>KeepAlive</key><true/>` keeps the process alive continuously; if it crashes or terminates, launchd restarts it.
   - Throttling: `<key>ThrottleInterval</key><integer>10</integer>` prevents spin-looping if the binary exits rapidly on startup.

3. **Standard Out/Error Logging:**
   - `<key>StandardOutPath</key><string>/Users/.../.config/pfmt/watch.stdout.log</string>`
   - `<key>StandardErrorPath</key><string>/Users/.../.config/pfmt/watch.stderr.log</string>`

4. **Environment Variables:**
   - launchd jobs do not inherit user shell profiles (`.zshrc` / `.bashrc`).
   - Define `<key>EnvironmentVariables</key>` with `PATH` to include standard binary paths (`/Users/.../.local/bin`, `/opt/homebrew/bin`, `/usr/bin`, etc.).
