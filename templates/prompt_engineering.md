# Template: Prompt Engineering & Formatting

- **File**: `templates/prompt_engineering.md`
- **Session Kind**: `write-enabled`
- **Primary Intent**: Authoring prompts, formatting constraints, prompt templates, system instructions, and schema definitions.

---

## 1. Needed Parts (Tags Analysis)

| Tag | Status | Mode | Rationale |
| :--- | :---: | :---: | :--- |
| `<task>` | **Required** | Dynamic | Target prompt, template, or formatting rule to create or enhance. |
| `<acceptance_criteria>` | **Required** | Dynamic | Schema compliant, single-responsibility tags, token-efficient, anti-hallucinational. |
| `<context>` | **Optional** | Dynamic | Target model capabilities, input token budgets, schema expectations. |
| `<scope>` | **Required** | Mixed | Target prompt files dynamic; pipeline orchestrator code out of scope. |
| `<rules>` | **Required** | Hardcoded | Anti-hallucination directives, single-responsibility tags, token economy, proper XML escaping. |
| `<verify>` | **Required** | Hardcoded Pattern | Validate XML tag pairing, check token count within budget, test dry-run. |
| `<report>` | **Required** | Hardcoded | Prompt Architecture, Token Budget & Model Target, Guardrails Enforced. |

---

## 2. Hardcoded vs Dynamic Data Analysis

### Static / Hardcoded Defaults
#### `scope > out_of_scope`
```text
Modifying agent orchestrator code or LLM client implementations unless explicitly requested.
```

#### `rules`
```text
- Single-responsibility tags: clear boundaries between task, criteria, rules, report.
- Explicit anti-hallucination directives: instruct agent to acknowledge unknown or unverified data.
- Token budget economy: keep instructions lean, avoiding conversational filler.
- Proper XML/Markdown escaping and tag nesting.
- Be EXTREMELY concise.
```

#### `verify`
```text
- Validate XML tag pairing and schema conformity.
- Estimate prompt token footprint against budget.
- Test prompt with sample user input in dry-run mode.
```

#### `report`
```text
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 📐 **Prompt Architecture**: (tags included and rationale)
2. 🎯 **Token Budget & Model Target**: (estimated token count)
3. 🛡️ **Guardrails Enforced**: (anti-hallucination and security boundaries)
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
Author / optimize prompt template for: {{prompt_target_or_use_case}}
</task>

<acceptance_criteria>
- Strict adherence to standardized XML tag schema
- All tags have clear single-responsibility definitions
- Contains explicit anti-hallucination and boundary guardrails
- Token-efficient and free of conversational fluff
</acceptance_criteria>

<context>
- Target model: {{target_model_name}}
- Token budget: {{token_budget}}
</context>

<scope>
<relevant_files>{{prompt_files}}</relevant_files>
<out_of_scope>Modifying agent orchestrator logic or client code</out_of_scope>
</scope>

<rules>
- Single-responsibility tags: clear boundaries between task, criteria, rules, report.
- Explicit anti-hallucination directives: instruct agent to acknowledge unknown or unverified data.
- Token budget economy: keep instructions lean, avoiding conversational filler.
- Proper XML/Markdown escaping and tag nesting.
- Be EXTREMELY concise.
</rules>

<verify>
- Validate XML tag pairing and syntax
- Verify token count fits within budget
</verify>

<report>
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 📐 **Prompt Architecture**: (tags included and rationale)
2. 🎯 **Token Budget & Model Target**: (estimated token count)
3. 🛡️ **Guardrails Enforced**: (anti-hallucination and security boundaries)
</report>
```
