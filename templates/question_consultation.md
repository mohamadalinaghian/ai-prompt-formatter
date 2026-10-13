# Template: Question & Consultation

- **File**: `templates/question_consultation.md`
- **Session Kind**: `read-only`
- **Primary Intent**: Answering architectural questions, explaining codebase mechanics, evaluating feasibility, or clarifying logic.

---

## 1. Needed Parts (Tags Analysis)

| Tag | Status | Mode | Rationale |
| :--- | :---: | :---: | :--- |
| `<task>` | **Required** | Dynamic | The specific question or topic to be answered/explained. |
| `<acceptance_criteria>` | **Required** | Dynamic | Clear direct answer, verified code citations, trade-off analysis if applicable. |
| `<context>` | **Optional** | Dynamic | Specific subsystem, file pointer, or user scenario triggering the question. |
| `<scope>` | **Optional** | Mixed | Target files to examine dynamically; hardcode out-of-scope as modifying files or code edits. |
| `<rules>` | **Required** | Hardcoded | Strict read-only analysis, ground explanations in code, cite file lines, no speculation/hallucination. |
| `<verify>` | **Omitted** | None | Not applicable: read-only consultation modifies no files. |
| `<report>` | **Required** | Hardcoded | Direct Answer, Code References, Key Considerations/Trade-offs. |

---

## 2. Hardcoded vs Dynamic Data Analysis

### Static / Hardcoded Defaults
#### `scope > out_of_scope`
```text
Making code edits, creating or deleting files, running destructive commands.
```

#### `rules`
```text
- Read-only analysis: do NOT make code edits or file modifications.
- Ground all explanations strictly in the codebase; cite file paths and line numbers.
- Anti-hallucination: if a fact is unverified or ambiguous, explicitly state it rather than assuming.
- Be EXTREMELY concise; provide the direct answer first, followed by rationale.
```

#### `report`
```text
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 💡 **Direct Answer**: (core conclusion or answer)
2. 📌 **Code References**: (file paths and line numbers)
3. ⚖️ **Key Considerations / Trade-offs**: (if applicable)
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
{{question_or_topic_to_explain}}
</task>

<acceptance_criteria>
- Provide a clear, direct answer to the core question
- Cite exact file paths and line ranges backing the explanation
- Highlight trade-offs or constraints if applicable
</acceptance_criteria>

<context>
- {{relevant_subsystem_or_background}}
</context>

<scope>
<relevant_files>{{relevant_files_to_inspect}}</relevant_files>
<out_of_scope>Making code edits, modifying repository files, destructive commands</out_of_scope>
</scope>

<rules>
- Read-only analysis: do NOT make code edits or file modifications.
- Ground all explanations strictly in the codebase; cite file paths and line numbers.
- Anti-hallucination: if a fact is unverified or ambiguous, explicitly state it rather than assuming.
- Be EXTREMELY concise; provide the direct answer first, followed by rationale.
</rules>

<report>
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 💡 **Direct Answer**: (core conclusion or answer)
2. 📌 **Code References**: (file paths and line numbers)
3. ⚖️ **Key Considerations / Trade-offs**: (if applicable)
</report>
```
