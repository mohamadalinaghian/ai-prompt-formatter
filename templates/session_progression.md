# Template: Session Progression & Approval

- **File**: `templates/session_progression.md`
- **Session Kind**: `write-enabled`
- **Primary Intent**: Continuing multi-step implementation plans ('continue to step 2', 'LGTM do it', 'approved proceed').

---

## 1. Needed Parts (Tags Analysis)

| Tag | Status | Mode | Rationale |
| :--- | :---: | :---: | :--- |
| `<task>` | **Required** | Dynamic | Specific step or phase being approved and resumed from the plan. |
| `<acceptance_criteria>` | **Required** | Dynamic | Current step exit check satisfied; baseline verified; step concluded. |
| `<context>` | **Required** | Dynamic | Active plan reference, prior step completion status, current git state. |
| `<scope>` | **Required** | Mixed | Files allocated for this step dynamic; subsequent unapproved steps out of scope. |
| `<rules>` | **Required** | Hardcoded | Strictly execute only current step, no jumping ahead, verify baseline and exit checks. |
| `<verify>` | **Required** | Dynamic/Step-specific | Run exact verification command defined in the plan for this step. |
| `<report>` | **Required** | Hardcoded | Step Completed, Key Changes Made, Exit Check Status, Ready for Next Step. |

---

## 2. Hardcoded vs Dynamic Data Analysis

### Static / Hardcoded Defaults
#### `scope > out_of_scope`
```text
Jumping ahead to subsequent plan steps without explicit user approval; modifying unrelated files.
```

#### `rules`
```text
- Strictly execute ONLY the designated step; do NOT run ahead to subsequent steps.
- Verify baseline before changing code; verify exit check before concluding.
- Adhere strictly to the approved plan without unilateral deviations.
- Be EXTREMELY concise. The user skims text.
```

#### `report`
```text
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 🪜 **Step Completed**: (step number and title)
2. 🎯 **Key Changes Made**: (files and logic changes)
3. ✅ **Exit Check Status**: (pass/fail against plan criteria)
4. ⏩ **Ready for Next Step**: (next actionable step awaiting user cue)
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
Execute Step {{step_number}}: {{step_title}}
</task>

<acceptance_criteria>
- Fulfill all goals specified in Step {{step_number}} of the approved plan
- Pass the explicit verification command for this step
- Satisfy the step exit check before stopping
</acceptance_criteria>

<context>
- Active plan: {{plan_reference_or_file}}
- Previous step status: {{previous_step_summary}}
</context>

<scope>
<relevant_files>{{files_for_this_step}}</relevant_files>
<out_of_scope>Subsequent plan steps not yet approved, modifying files outside this step's scope</out_of_scope>
</scope>

<rules>
- Strictly execute ONLY the designated step; do NOT run ahead to subsequent steps.
- Verify baseline before changing code; verify exit check before concluding.
- Adhere strictly to the approved plan without unilateral deviations.
- Be EXTREMELY concise. The user skims text.
</rules>

<verify>
- Run baseline check: `{{step_baseline_command}}`
- Run step verification command: `{{step_verification_command}}`
</verify>

<report>
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 🪜 **Step Completed**: (step number and title)
2. 🎯 **Key Changes Made**: (files and logic changes)
3. ✅ **Exit Check Status**: (pass/fail against plan criteria)
4. ⏩ **Ready for Next Step**: (next actionable step awaiting user cue)
</report>
```
