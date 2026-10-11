# Research: Karabiner-Elements Complex Modifications

- **Source URL:** https://karabiner-elements.pqrs.org/docs/json/complex-modifications-manipulator-definition/to/shell-command/
- **Access Date:** 2026-10-11
- **Target:** macOS Ventura, Sonoma, Sequoia

## Findings

1. **`shell_command` execution context:**
   - Karabiner executes `shell_command` using `env -i sh -c "..."`.
   - Only minimal environment variables (`$HOME`, `$UID`, `$USER`) are inherited.
   - `$PATH` does not contain user additions like `~/.cargo/bin`, `~/.local/bin`, or `/opt/homebrew/bin` by default.
   - Therefore, wrapper scripts invoked from Karabiner should either explicitly export `PATH="$HOME/.local/bin:$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"` or be invoked with an absolute path.
   - Locale should be set: `export LC_ALL=en_US.UTF-8`.

2. **Modifier Release:**
   - When a shortcut (e.g. `Cmd + Shift + I` or `Ctrl + Option + P`) triggers a `shell_command`, the physical keys may still be held down for several hundred milliseconds.
   - If the script simulates `Cmd+A` or `Cmd+C` immediately, keystrokes can collide with user-held modifiers.
   - Standard workaround: Introduce a small debounce sleep (`sleep 0.4`) before sending keystrokes.

3. **Complex Modifications Structure:**
   ```json
   {
     "title": "pfmt AI Prompt Enhancer",
     "rules": [
       {
         "description": "Format prompt with pfmt (Ctrl + Option + P)",
         "manipulators": [
           {
             "type": "basic",
             "from": {
               "key_code": "p",
               "modifiers": {
                 "mandatory": ["control", "option"]
               }
             },
             "to": [
               {
                 "shell_command": "/bin/bash ~/.local/bin/ai_run.sh"
               }
             ]
           }
         ]
       }
     ]
   }
   ```
