# Template: Documentation & Guidelines

- **File**: `templates/documentation.md`
- **Session Kind**: `write-enabled`
- **Primary Intent**: Writing or updating AGENTS.md, README files, API documentation, architecture design records, and user manuals.

---

## 1. Needed Parts (Tags Analysis)

| Tag | Status | Mode | Rationale |
| :--- | :---: | :---: | :--- |
| `<task>` | **Required** | Dynamic | Specific doc file or section to write or update. |
| `<acceptance_criteria>` | **Required** | Dynamic | Matches current codebase reality, tested code snippets, no broken links, skim-friendly. |
| `<context>` | **Optional** | Dynamic | Source code symbols, CLI commands, architecture decisions to document. |
| `<scope>` | **Required** | Mixed | Target doc files dynamic; source code and tests out of scope. |
| `<rules>` | **Required** | Hardcoded | Documentation integrity, test commands before documenting, GitHub Markdown, clickable file links. |
| `<verify>` | **Required** | Hardcoded Pattern | Verify file links exist, test documented CLI commands execute successfully. |
| `<report>` | **Required** | Hardcoded | Documentation Updated, Verified References, Maintenance Notes. |

---

## 2. Hardcoded vs Dynamic Data Analysis

### Static / Hardcoded Defaults
#### `scope > out_of_scope`
```text
Modifying application source code, refactoring tests, altering unrelated documentation sections.
```

#### `rules`
```text
- Documentation integrity: maintain existing docstrings and comments unless explicitly instructed.
- Verify code snippets: all commands, flags, and code snippets in docs must be tested and accurate.
- Keep docs concise, skim-friendly, and well-structured with GitHub Markdown.
- Use clickable links with `file://` scheme for file and symbol references.
- Be EXTREMELY concise.
```

#### `verify`
```text
- Verify all referenced file paths exist in the repository.
- Run every documented CLI command to ensure exit code 0.
- Check markdown formatting and link integrity.
```

#### `report`
```text
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 📚 **Documentation Updated**: (files and sections modified)
2. 🔗 **Verified References**: (code paths and commands verified)
3. 💡 **Maintenance Notes**: (guidelines for future doc sync)
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
Update / create documentation for: {{documentation_target}}
</task>

<acceptance_criteria>
- Documentation accurately reflects current codebase implementation
- All CLI commands and code snippets verified to execute successfully
- Clean GitHub Markdown formatting with clickable file links
- Skim-friendly structure with bullet points and clear headings
</acceptance_criteria>

<context>
- Target audience / doc file: {{target_doc_file}}
- Technical details to incorporate: {{technical_details}}
</context>

<scope>
<relevant_files>{{doc_files}}</relevant_files>
<out_of_scope>Modifying application source code or refactoring tests</out_of_scope>
</scope>

<rules>
- Documentation integrity: maintain existing docstrings and comments unless explicitly instructed.
- Verify code snippets: all commands, flags, and code snippets in docs must be tested and accurate.
- Keep docs concise, skim-friendly, and well-structured with GitHub Markdown.
- Use clickable links with `file://` scheme for file and symbol references.
- Be EXTREMELY concise.
</rules>

<verify>
- Verify all referenced file paths exist in repository
- Execute documented CLI commands in a subshell to verify success
</verify>

<report>
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 📚 **Documentation Updated**: (files and sections modified)
2. 🔗 **Verified References**: (code paths and commands verified)
3. 💡 **Maintenance Notes**: (guidelines for future doc sync)
</report>
```
