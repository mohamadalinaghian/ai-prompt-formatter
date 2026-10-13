# Template: Testing & Verification

- **File**: `templates/testing_verification.md`
- **Session Kind**: `write-enabled`
- **Primary Intent**: Authoring unit, integration, or end-to-end tests, expanding test coverage, and validating edge cases.

---

## 1. Needed Parts (Tags Analysis)

| Tag | Status | Mode | Rationale |
| :--- | :---: | :---: | :--- |
| `<task>` | **Required** | Dynamic | Target modules, functions, or workflows to test. |
| `<acceptance_criteria>` | **Required** | Dynamic | Test cases written; edge cases covered; assertions test real behavior; 100% pass. |
| `<context>` | **Optional** | Dynamic | Test framework, existing test utilities, fixtures, mocking rules. |
| `<scope>` | **Required** | Mixed | Test files dynamic; modifying business logic out-of-scope unless approved for testability. |
| `<rules>` | **Required** | Hardcoded | Real behavior assertions, cover edge cases, deterministic tests, never weaken existing tests. |
| `<verify>` | **Required** | Hardcoded Pattern | Run new tests, verify assertion failure on inverted expectation, run workspace suite. |
| `<report>` | **Required** | Hardcoded | Tests Added, Execution Results, Edge Cases Covered. |

---

## 2. Hardcoded vs Dynamic Data Analysis

### Static / Hardcoded Defaults
#### `scope > out_of_scope`
```text
Modifying core production logic unless adding approved testability hooks; weakening existing assertions.
```

#### `rules`
```text
- Test real behavior; avoid brittle tautological assertions or mock-only tests.
- Cover happy path, boundary values, error paths, and invalid inputs.
- Do not alter or weaken existing passing tests to accommodate failures.
- Deterministic tests only: no race conditions, sleep delays, or flaky external dependencies.
- Be EXTREMELY concise.
```

#### `verify`
```text
- Run targeted test: `cargo test -p <pkg> <test_name>`.
- Verify test fails when expectation is inverted.
- Run full workspace test suite: `cargo test --workspace -- --test-threads=1`.
- Run linter: `cargo clippy --workspace --all-targets -- -D warnings`.
```

#### `report`
```text
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 🧪 **Tests Added / Updated**: (test functions and scenarios)
2. ✅ **Test Execution Results**: (pass count, timing)
3. 🛡️ **Edge Cases Covered**: (boundary and failure cases tested)
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
Add comprehensive tests for: {{target_module_or_feature}}
</task>

<acceptance_criteria>
- Tests cover happy path, error paths, and boundary conditions
- All newly added and existing tests pass consistently
- No flaky tests or arbitrary sleep statements introduced
</acceptance_criteria>

<context>
- Testing framework & tools: {{test_framework_and_fixtures}}
- Target behaviors to validate: {{behaviors_to_validate}}
</context>

<scope>
<relevant_files>{{test_files}}</relevant_files>
<out_of_scope>Modifying production business logic, weakening existing tests</out_of_scope>
</scope>

<rules>
- Test real behavior; avoid brittle tautological assertions or mock-only tests.
- Cover happy path, boundary values, error paths, and invalid inputs.
- Do not alter or weaken existing passing tests to accommodate failures.
- Deterministic tests only: no race conditions, sleep delays, or flaky external dependencies.
- Be EXTREMELY concise.
</rules>

<verify>
- Run targeted test suite: `{{targeted_test_command}}`
- Run full workspace tests: `cargo test --workspace -- --test-threads=1`
- Run linter: `cargo clippy --workspace --all-targets -- -D warnings`
</verify>

<report>
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 🧪 **Tests Added / Updated**: (test functions and scenarios)
2. ✅ **Test Execution Results**: (pass count, timing)
3. 🛡️ **Edge Cases Covered**: (boundary and failure cases tested)
</report>
```
