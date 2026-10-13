# Template: Report & Status Summary

- **File**: `templates/report_summary.md`
- **Session Kind**: `read-only`
- **Primary Intent**: Compiling progress reports, project status summaries, feature readiness assessments, and changelogs.

---

## 1. Needed Parts (Tags Analysis)

| Tag | Status | Mode | Rationale |
| :--- | :---: | :---: | :--- |
| `<task>` | **Required** | Dynamic | Target topic or milestone to report on. |
| `<acceptance_criteria>` | **Required** | Dynamic | Factual summary based on repository state, test status, or git history. |
| `<context>` | **Optional** | Dynamic | Milestones, active issues, or timeframe covered. |
| `<scope>` | **Required** | Hardcoded | Modifying code or committing changes is out of scope. |
| `<rules>` | **Required** | Hardcoded | Read-only, factual evidence only, no speculative percentages, concise skim-friendly formatting. |
| `<verify>` | **Omitted** | None | Purely informational status generation. |
| `<report>` | **Required** | Hardcoded | Progress & Status, Blockers & Risks, Next Action Items. |

---

## 2. Hardcoded vs Dynamic Data Analysis

### Static / Hardcoded Defaults
#### `scope > out_of_scope`
```text
Making code edits, committing files, mutating project state.
```

#### `rules`
```text
- Read-only: do NOT make code edits or file modifications.
- All status statements must be verified against actual files, git logs, or test runs.
- No speculative or fabricated metrics.
- Be EXTREMELY concise; use bullet points and tables.
```

#### `report`
```text
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 📊 **Progress & Current Status**: (factual completed deliverables)
2. ⚠️ **Blockers & High-Priority Risks**: (active impediments)
3. 📋 **Next Action Items**: (prioritized next steps)
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
Compile a status / progress report on: {{report_target_or_milestone}}
</task>

<acceptance_criteria>
- Report accurately reflects current codebase, git history, and test status
- Status items supported by verified facts, not speculative estimates
- Identifies blockers, unfulfilled requirements, and pending items
</acceptance_criteria>

<context>
- Milestone / Goal: {{milestone_or_target_goal}}
- Relevant logs or history: {{history_or_logs_context}}
</context>

<scope>
<out_of_scope>Making code changes, committing files, mutating project state</out_of_scope>
</scope>

<rules>
- Read-only: do NOT make code edits or file modifications.
- All status statements must be verified against actual files, git logs, or test runs.
- No speculative or fabricated metrics.
- Be EXTREMELY concise; use bullet points and tables.
</rules>

<report>
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 📊 **Progress & Current Status**: (factual completed deliverables)
2. ⚠️ **Blockers & High-Priority Risks**: (active impediments)
3. 📋 **Next Action Items**: (prioritized next steps)
</report>
```
