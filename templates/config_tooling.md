# Template: Configuration & Tooling Setup

- **File**: `templates/config_tooling.md`
- **Session Kind**: `write-enabled`
- **Primary Intent**: Configuring developer tools, MCP servers, linters, Karabiner bindings, launchd services, or editor settings.

---

## 1. Needed Parts (Tags Analysis)

| Tag | Status | Mode | Rationale |
| :--- | :---: | :---: | :--- |
| `<task>` | **Required** | Dynamic | Target tool, MCP server, linter, or config file to install/configure. |
| `<acceptance_criteria>` | **Required** | Dynamic | Config syntax valid, tool executable, settings preserved, passes diagnostic check. |
| `<context>` | **Dynamic** | Dynamic | OS (macOS), tool version, path locations, environment variables. |
| `<scope>` | **Required** | Mixed | Target config files dynamic; application source code out of scope. |
| `<rules>` | **Required** | Hardcoded | Preserve comments and unrelated settings, respect macOS paths, validate syntax before saving. |
| `<verify>` | **Required** | Hardcoded Pattern | Run tool test command, lint configuration file (`plutil -lint`, `toml lint`). |
| `<report>` | **Required** | Hardcoded | Configured Tools, Files Modified, Verification Output. |

---

## 2. Hardcoded vs Dynamic Data Analysis

### Static / Hardcoded Defaults
#### `scope > out_of_scope`
```text
Modifying application source code or unrelated tool configurations; hardcoding machine-specific absolute paths.
```

#### `rules`
```text
- Preserve existing comments and unrelated settings in configuration files.
- Respect OS-specific paths and conventions (macOS standards, ~/.config, LaunchAgents).
- Validate syntax (JSON/TOML/YAML/plist) before saving.
- No hardcoded machine-specific absolute paths unless mandatory.
- Be EXTREMELY concise.
```

#### `verify`
```text
- Run tool invocation or version check: `{{tool_binary}} --version`.
- Validate configuration file syntax (`plutil -lint`, `cargo run -p pfmt-cli -- doctor`, etc.).
```

#### `report`
```text
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. ⚙️ **Configured Tools**: (tool name, version, status)
2. 📂 **Files Modified**: (configuration file paths)
3. 🧪 **Verification Output**: (output from CLI test command)
```

### Dynamic Runtime Inputs
The following placeholders are populated dynamically from user input, context builder, or verifier:
- `{{task}}`, sub-elements, or criteria
- `{{acceptance_criteria}}`, sub-elements, or criteria
- `{{context}}`, sub-elements, or criteria
- `{{scope}}`, sub-elements, or criteria

---

## 3. Standardized XML Prompt Template

```xml
<task>
Configure tool / environment: {{tool_or_config_target}}
</task>

<acceptance_criteria>
- Configuration file syntax parses without errors
- Tool or service successfully executes and responds
- Existing configuration settings and comments are preserved
</acceptance_criteria>

<context>
- Operating system: macOS
- Configuration path: {{config_file_path}}
- Required tool version / parameters: {{tool_parameters}}
</context>

<scope>
<relevant_files>{{config_files}}</relevant_files>
<out_of_scope>Modifying application source code or unrelated tool configurations</out_of_scope>
</scope>

<rules>
- Preserve existing comments and unrelated settings in configuration files.
- Respect OS-specific paths and conventions (macOS standards, ~/.config, LaunchAgents).
- Validate syntax (JSON/TOML/YAML/plist) before saving.
- No hardcoded machine-specific absolute paths unless mandatory.
- Be EXTREMELY concise.
</rules>

<verify>
- Run tool version / help command: `{{tool_command_check}}`
- Validate configuration file syntax: `{{syntax_validation_command}}`
</verify>

<report>
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. ⚙️ **Configured Tools**: (tool name, version, status)
2. 📂 **Files Modified**: (configuration file paths)
3. 🧪 **Verification Output**: (output from CLI test command)
</report>
```
