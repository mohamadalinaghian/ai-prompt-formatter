# Template: Implementation & Refactoring

- **File**: `templates/implementation_refactoring.md`
- **Session Kind**: `write-enabled`
- **Primary Intent**: Authoring new features, writing code, refactoring modules, adding API endpoints, or updating UI components.

---

## 1. Needed Parts (Tags Analysis)

| Tag | Status | Mode | Rationale |
| :--- | :---: | :---: | :--- |
| `<task>` | **Required** | Dynamic | Specific feature or refactoring objective with exact naming, routes, and behavior. |
| `<acceptance_criteria>` | **Required** | Dynamic | Observable, testable criteria for completion including edge cases. |
| `<context>` | **Optional** | Dynamic | Relevant interfaces, architectural guidelines, caller code, or dependencies. |
| `<scope>` | **Required** | Mixed | Must specify relevant files dynamically; out-of-scope can hardcode unapproved refactorings and public API breakages. |
| `<rules>` | **Required** | Hardcoded | Codebase patterns, no mock data, ask before breaking APIs, preserve comments, preserve test integrity. |
| `<verify>` | **Required** | Hardcoded Pattern | Standard build, test, and lint commands (`cargo check`, `cargo test`, `cargo clippy`). |
| `<report>` | **Required** | Hardcoded | Structured concise report format (Key Changes, Blockers, Remaining Tasks). |

---

## 2. Hardcoded vs Dynamic Data Analysis

### Static / Hardcoded Defaults
#### `scope > out_of_scope`
```text
Unrelated modules, breaking public API changes without prior approval, premature optimization, rewriting passing tests.
```

#### `rules`
```text
- Be EXTREMELY concise. The user skims text. Do not explain how code works unless asked.
- Follow existing patterns in the repo; check neighboring files first.
- No mock data or swallowed errors; handle failures explicitly.
- Ask before: destructive actions, schema or public API changes.
- Preserve existing comments and docstrings unrelated to your changes.
- Never delete or weaken existing tests to make a build pass.
- For other ambiguity, pick the most reasonable option and list it in the report.
```

#### `verify`
```text
- Run type check and build (`cargo check` or project build command).
- Run test suite (`cargo test --workspace -- --test-threads=1` or project test command).
- Run linter and formatter check (`cargo clippy`, `cargo fmt --check`).
```

#### `report`
```text
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 🎯 **Key Changes**: (bullet points, max 1 line each)
2. ⚠️ **Important / Blockers**: (assumptions, blockers, or critical decisions)
3. 📋 **Remaining Tasks**: (what is left to do, actionable items)
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
{{task_description}}
</task>

<acceptance_criteria>
- {{criterion_1}}
- {{criterion_2}}
- {{edge_case_handling}}
</acceptance_criteria>

<context>
- {{architecture_or_dependency_context}}
- {{caller_or_integration_details}}
</context>

<scope>
<relevant_files>{{relevant_files}}</relevant_files>
<out_of_scope>Unrelated modules, breaking public API changes without prior approval, rewriting passing tests</out_of_scope>
</scope>

<rules>
- Be EXTREMELY concise. The user skims text. Do not explain how code works unless asked.
- Follow existing patterns in the repo; check neighboring files first.
- No mock data or swallowed errors; handle failures explicitly.
- Ask before: destructive actions, schema or public API changes.
- Preserve existing comments and docstrings unrelated to your changes.
- Never delete or weaken existing tests to make a build pass.
- For other ambiguity, pick the most reasonable option and list it in the report.
</rules>

<verify>
- Run type check and build (`cargo check` or equivalent build command)
- Run unit and integration tests (`cargo test --workspace -- --test-threads=1`)
- Run linter and formatter check (`cargo clippy`, `cargo fmt --check`)
</verify>

<report>
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 🎯 **Key Changes**: (bullet points, max 1 line each)
2. ⚠️ **Important / Blockers**: (assumptions, blockers, or critical decisions)
3. 📋 **Remaining Tasks**: (what is left to do, actionable items)
</report>
```
