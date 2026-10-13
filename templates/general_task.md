# Template: General Instruction / Task

- **File**: `templates/general_task.md`
- **Session Kind**: `write-enabled`
- **Primary Intent**: Executing multi-faceted tasks, custom workflows, or instructions outside standard categories.

---

## 1. Needed Parts (Tags Analysis)

| Tag | Status | Mode | Rationale |
| :--- | :---: | :---: | :--- |
| `<task>` | **Required** | Dynamic | Imperative task description and objective. |
| `<acceptance_criteria>` | **Required** | Dynamic | Observable outcomes and conditions of satisfaction. |
| `<context>` | **Optional** | Dynamic | Relevant background, environment details, or preconditions. |
| `<scope>` | **Required** | Mixed | Target files dynamic; destructive actions and unrelated files out of scope. |
| `<rules>` | **Required** | Hardcoded | Concise, check patterns, no mock data, ask before destructive changes. |
| `<verify>` | **Required** | Hardcoded Pattern | Run workspace test suite and linter before declaring complete. |
| `<report>` | **Required** | Hardcoded | Key Changes, Important/Blockers, Remaining Tasks. |

---

## 2. Hardcoded vs Dynamic Data Analysis

### Static / Hardcoded Defaults
#### `scope > out_of_scope`
```text
Unrelated modules, destructive changes without approval, breaking public API contracts.
```

#### `rules`
```text
- Be EXTREMELY concise. The user skims text. Do not explain how code works unless asked.
- Follow existing patterns in the repo; check neighboring files first.
- No mock data or swallowed errors; handle failures explicitly.
- Ask before: destructive actions, schema or public API changes.
- For other ambiguity, pick the most reasonable option and list it in the report.
```

#### `verify`
```text
- Run workspace test suite: `cargo test --workspace -- --test-threads=1`.
- Run linter and formatting checks: `cargo clippy --workspace --all-targets -- -D warnings`.
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
{{task_instruction}}
</task>

<acceptance_criteria>
- {{criterion_1}}
- {{criterion_2}}
</acceptance_criteria>

<context>
- {{context_or_preconditions}}
</context>

<scope>
<relevant_files>{{relevant_files}}</relevant_files>
<out_of_scope>Unrelated modules, destructive changes without approval, breaking public APIs</out_of_scope>
</scope>

<rules>
- Be EXTREMELY concise. The user skims text. Do not explain how code works unless asked.
- Follow existing patterns in the repo; check neighboring files first.
- No mock data or swallowed errors; handle failures explicitly.
- Ask before: destructive actions, schema or public API changes.
- For other ambiguity, pick the most reasonable option and list it in the report.
</rules>

<verify>
- Run workspace test suite: `cargo test --workspace -- --test-threads=1`
- Run linter: `cargo clippy --workspace --all-targets -- -D warnings`
</verify>

<report>
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 🎯 **Key Changes**: (bullet points, max 1 line each)
2. ⚠️ **Important / Blockers**: (assumptions, blockers, or critical decisions)
3. 📋 **Remaining Tasks**: (what is left to do, actionable items)
</report>
```
