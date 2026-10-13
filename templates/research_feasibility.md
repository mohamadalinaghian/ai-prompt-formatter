# Template: Research & Feasibility

- **File**: `templates/research_feasibility.md`
- **Session Kind**: `read-only`
- **Primary Intent**: Investigating third-party libraries, tool options, feasibility of technical approaches, and trade-off matrices.

---

## 1. Needed Parts (Tags Analysis)

| Tag | Status | Mode | Rationale |
| :--- | :---: | :---: | :--- |
| `<task>` | **Required** | Dynamic | Target library, tool, or approach to evaluate. |
| `<acceptance_criteria>` | **Required** | Dynamic | Evaluation matrix, compatibility verdict, license check, clear recommendation. |
| `<context>` | **Required** | Dynamic | Current project stack, runtime constraints, performance budgets. |
| `<scope>` | **Required** | Hardcoded | Modifying repository code or committing prototypes is out of scope. |
| `<rules>` | **Required** | Hardcoded | Read-only, check stack compatibility and license terms, distinguish verified facts from marketing. |
| `<verify>` | **Optional** | Hardcoded Pattern | Verify package versions, compatibility with Rust/Node edition, and license compatibility. |
| `<report>` | **Required** | Hardcoded | Evaluation Summary, Recommendation, Compatibility & Risks. |

---

## 2. Hardcoded vs Dynamic Data Analysis

### Static / Hardcoded Defaults
#### `scope > out_of_scope`
```text
Modifying repository code, installing untrusted dependencies, committing prototypes.
```

#### `rules`
```text
- Read-only research: do not alter workspace files.
- Evaluate compatibility with existing stack (Rust edition, OS support, licenses).
- Distinguish verified capabilities from marketing claims.
- Present trade-offs objectively; be EXTREMELY concise.
```

#### `verify`
```text
- Verify dependency compatibility with project dependencies and toolchain.
- Check open issue tracker and maintenance health of candidate libraries.
```

#### `report`
```text
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. ⚖️ **Evaluation Summary**: (options evaluated, pros & cons)
2. 🎯 **Recommendation**: (chosen option with technical justification)
3. ⚠️ **Compatibility & Risks**: (potential blockers, license traps)
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
Research feasibility and options for: {{research_topic_or_tool_options}}
</task>

<acceptance_criteria>
- Evaluate candidate options against project constraints
- Check licensing, maintenance health, and platform compatibility
- Provide a clear go/no-go recommendation with trade-offs
</acceptance_criteria>

<context>
- Current project stack: {{current_stack_and_dependencies}}
- Performance / Size budget: {{constraints_and_budgets}}
</context>

<scope>
<out_of_scope>Modifying repository code, installing untrusted dependencies, committing prototypes</out_of_scope>
</scope>

<rules>
- Read-only research: do not alter workspace files.
- Evaluate compatibility with existing stack (Rust edition, OS support, licenses).
- Distinguish verified capabilities from marketing claims.
- Present trade-offs objectively; be EXTREMELY concise.
</rules>

<verify>
- Verify dependency compatibility with project dependencies and toolchain
- Check open issue tracker and maintenance health of candidate libraries
</verify>

<report>
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. ⚖️ **Evaluation Summary**: (options evaluated, pros & cons)
2. 🎯 **Recommendation**: (chosen option with technical justification)
3. ⚠️ **Compatibility & Risks**: (potential blockers, license traps)
</report>
```
