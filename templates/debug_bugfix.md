# Template: Debug & Bug Fix

- **File**: `templates/debug_bugfix.md`
- **Session Kind**: `write-enabled`
- **Primary Intent**: Pinpointing root cause and fixing compile errors, runtime crashes, logic bugs, or regressions.

---

## 1. Needed Parts (Tags Analysis)

| Tag | Status | Mode | Rationale |
| :--- | :---: | :---: | :--- |
| `<task>` | **Required** | Dynamic | Specific error description, stack trace, failing test name, or bug symptom. |
| `<acceptance_criteria>` | **Required** | Dynamic | Root cause identified; failing case passes; regression test added; no side effects. |
| `<context>` | **Required** | Dynamic | Stack trace, error logs, reproduction steps, recent commits, or environment details. |
| `<scope>` | **Required** | Mixed | Failing source files dynamically specified; cosmetic changes and broad refactorings out of scope. |
| `<rules>` | **Required** | Hardcoded | Root cause first, minimal blast radius, never suppress errors, regression test mandatory. |
| `<verify>` | **Required** | Hardcoded Pattern | Reproduce bug before fix, run targeted regression test, run full suite for zero regression. |
| `<report>` | **Required** | Hardcoded | Root cause, fix applied, verification test output. |

---

## 2. Hardcoded vs Dynamic Data Analysis

### Static / Hardcoded Defaults
#### `scope > out_of_scope`
```text
Cosmetic refactorings, style/formatting changes in unrelated files, redesigning surrounding architectures.
```

#### `rules`
```text
- Pinpoint root cause before modifying code; verify the failure mechanism first.
- Minimal blast radius: touch only what is strictly necessary to resolve the bug.
- Never suppress or swallow errors with empty catch blocks or loose typing.
- Always add or update a regression test verifying the bug cannot recur.
- Be EXTREMELY concise. The user skims text.
```

#### `verify`
```text
- Reproduce failure prior to fix to verify test failure.
- Run target test command: `cargo test -p <pkg> <test_name>`.
- Run full workspace tests to guarantee zero regressions: `cargo test --workspace -- --test-threads=1`.
- Run linter: `cargo clippy --workspace --all-targets -- -D warnings`.
```

#### `report`
```text
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 🔍 **Root Cause**: (exact line and failure mechanism)
2. 🎯 **Fix Applied**: (specific files and logic changed)
3. 🧪 **Verification Result**: (test command and pass confirmation)
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
Fix bug / failure: {{bug_description_or_error_message}}
</task>

<acceptance_criteria>
- Root cause identified and explained concisely
- The failing scenario passes consistently
- Regression test added or existing test updated
- Zero regressions across existing test suite
</acceptance_criteria>

<context>
- Error log / stack trace: {{stack_trace_or_error_log}}
- Reproduction steps: {{reproduction_steps}}
</context>

<scope>
<relevant_files>{{relevant_files}}</relevant_files>
<out_of_scope>Cosmetic refactorings, style changes in unrelated files, architecture redesigns</out_of_scope>
</scope>

<rules>
- Pinpoint root cause before modifying code; verify the failure mechanism first.
- Minimal blast radius: touch only what is strictly necessary to resolve the bug.
- Never suppress or swallow errors with empty catch blocks or loose typing.
- Always add or update a regression test verifying the bug cannot recur.
- Be EXTREMELY concise. The user skims text.
</rules>

<verify>
- Reproduce failure prior to fix to verify test failure
- Run target regression test: `{{target_test_command}}`
- Run full workspace test suite: `cargo test --workspace -- --test-threads=1`
- Run linter: `cargo clippy --workspace --all-targets -- -D warnings`
</verify>

<report>
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 🔍 **Root Cause**: (exact line and failure mechanism)
2. 🎯 **Fix Applied**: (specific files and logic changed)
3. 🧪 **Verification Result**: (test command and pass confirmation)
</report>
```
