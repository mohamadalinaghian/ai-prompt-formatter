# Template: Plan & Architecture

- **File**: `templates/plan_architecture.md`
- **Session Kind**: `read-only (planning phase)`
- **Primary Intent**: Designing software architecture, database schemas, or authoring step-by-step implementation roadmaps.

---

## 1. Needed Parts (Tags Analysis)

| Tag | Status | Mode | Rationale |
| :--- | :---: | :---: | :--- |
| `<task>` | **Required** | Dynamic | System design or roadmap to construct. |
| `<acceptance_criteria>` | **Required** | Dynamic | Atomic sequential phases, verification commands per step, rollback plan, explicit baseline. |
| `<context>` | **Required** | Dynamic | System architecture, existing dependencies, constraints, project requirements. |
| `<scope>` | **Required** | Hardcoded | Modifying production code during planning is out of scope. |
| `<rules>` | **Required** | Hardcoded | Planning only, atomic steps, define verification per step, flag high-risk/breaking changes. |
| `<verify>` | **Required** | Hardcoded Pattern | Verify referenced files and types exist in repo; verify proposed commands are executable. |
| `<report>` | **Required** | Hardcoded | Architecture Overview, Step-by-Step Breakdown, Risks & Open Questions. |

---

## 2. Hardcoded vs Dynamic Data Analysis

### Static / Hardcoded Defaults
#### `scope > out_of_scope`
```text
Editing source code, creating production files, implementing features ahead of plan approval.
```

#### `rules`
```text
- Pure planning phase: DO NOT modify any code or configuration files.
- Divide into small, atomic, independently testable phases (Step 0: Baseline, Step 1..N).
- Every step must specify: Goal, files touched, verification command, and exit criteria.
- Flag high-risk changes, database migrations, and public API breaking changes explicitly.
- Ask before assuming architectural decisions or external dependencies.
```

#### `verify`
```text
- Verify all referenced files, structs, and functions exist in the repository.
- Validate feasibility and flags of proposed CLI commands against project environment.
```

#### `report`
```text
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 🏛️ **Architecture / Plan Overview**: (1-2 sentences)
2. 🪜 **Step-by-Step Breakdown**: (atomic phases with verification commands)
3. ⚠️ **Risks & Open Questions**: (decisions needing user confirmation)
```

### Dynamic Runtime Inputs
The following placeholders are populated dynamically from user input, context builder, or verifier:
- `{{task}}`, sub-elements, or criteria
- `{{acceptance_criteria}}`, sub-elements, or criteria
- `{{context}}`, sub-elements, or criteria

---

## 3. Standardized XML Prompt Template

```xml
<task>
Create an implementation plan for: {{feature_or_refactor_goal}}
</task>

<acceptance_criteria>
- Plan starts with Step 0: Baseline (current test/build status)
- Steps are atomic, ordered, and independently verifiable
- Each step defines: Goal, Files to touch, Exact verification command, Exit criteria
- Identifies potential breaking changes and rollback strategy
</acceptance_criteria>

<context>
- Tech stack: {{tech_stack}}
- Existing architecture: {{architecture_context}}
- Constraints: {{constraints_and_budgets}}
</context>

<scope>
<out_of_scope>Writing production code or modifying files during this planning phase</out_of_scope>
</scope>

<rules>
- Pure planning phase: DO NOT modify any code or configuration files.
- Divide into small, atomic, independently testable phases (Step 0: Baseline, Step 1..N).
- Every step must specify: Goal, files touched, verification command, and exit criteria.
- Flag high-risk changes, database migrations, and public API breaking changes explicitly.
- Ask before assuming architectural decisions or external dependencies.
</rules>

<verify>
- Verify all referenced files, structs, and functions exist in the repository
- Validate feasibility and flags of proposed CLI commands against project environment
</verify>

<report>
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 🏛️ **Architecture / Plan Overview**: (1-2 sentences)
2. 🪜 **Step-by-Step Breakdown**: (atomic phases with verification commands)
3. ⚠️ **Risks & Open Questions**: (decisions needing user confirmation)
</report>
```
