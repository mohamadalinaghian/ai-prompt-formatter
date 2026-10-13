# Template: Codebase Inspection & Audit

- **File**: `templates/codebase_audit.md`
- **Session Kind**: `read-only`
- **Primary Intent**: Security audits, vulnerability scans, memory leak checks, permission audits, and code quality sweeps.

---

## 1. Needed Parts (Tags Analysis)

| Tag | Status | Mode | Rationale |
| :--- | :---: | :---: | :--- |
| `<task>` | **Required** | Dynamic | Target subsystem and audit focus (security, permissions, memory, concurrency). |
| `<acceptance_criteria>` | **Required** | Dynamic | Exhaustive review, verified exploitability/reachability, prioritized findings. |
| `<context>` | **Required** | Dynamic | Threat model, permission policies, performance budgets, critical paths. |
| `<scope>` | **Required** | Mixed | Target source directories dynamic; modifying code is strictly out of scope. |
| `<rules>` | **Required** | Hardcoded | Read-only, zero false positives, classify by severity, cite exact file and line ranges. |
| `<verify>` | **Required** | Hardcoded Pattern | Trace call sites from entry point to sink to prove reachability. |
| `<report>` | **Required** | Hardcoded | Audit Findings (Severity, File:Line), Impact Assessment, Remediation Steps. |

---

## 2. Hardcoded vs Dynamic Data Analysis

### Static / Hardcoded Defaults
#### `scope > out_of_scope`
```text
Applying code changes or refactoring during the audit phase; speculative unverified reports.
```

#### `rules`
```text
- Read-only: DO NOT modify any code or files.
- Zero false-positive policy: verify exploitability or reachability before reporting.
- Classify findings by severity: Critical, High, Medium, Low, Info.
- Include exact file paths and line ranges for every finding.
- Be EXTREMELY concise; no fluff.
```

#### `verify`
```text
- Trace execution paths from public entry points to affected code to confirm reachability.
- Verify syntax and patterns with ripgrep/AST inspection.
```

#### `report`
```text
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 🛡️ **Audit Findings**: (Severity, File:Line, Description)
2. 💥 **Impact & Exploitability**: (realistic risk assessment)
3. 🔧 **Remediation Steps**: (actionable recommendations for each finding)
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
Audit codebase for: {{audit_focus_area_e.g._security_permissions_memory}}
</task>

<acceptance_criteria>
- Scan targeted modules exhaustively for vulnerabilities or anti-patterns
- Each finding verified for reachability with exact file and line numbers
- Findings classified by severity (Critical / High / Medium / Low / Info)
- Concrete remediation steps provided for each finding
</acceptance_criteria>

<context>
- Threat model / Policy: {{threat_model_or_rules}}
- Entry points: {{entry_points}}
</context>

<scope>
<relevant_files>{{directories_or_modules_to_audit}}</relevant_files>
<out_of_scope>Modifying files or applying code fixes during the audit phase</out_of_scope>
</scope>

<rules>
- Read-only: DO NOT modify any code or files.
- Zero false-positive policy: verify exploitability or reachability before reporting.
- Classify findings by severity: Critical, High, Medium, Low, Info.
- Include exact file paths and line ranges for every finding.
- Be EXTREMELY concise; no fluff.
</rules>

<verify>
- Trace execution paths from public entry points to affected code to confirm reachability
- Verify syntax and patterns with ripgrep/AST inspection
</verify>

<report>
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 🛡️ **Audit Findings**: (Severity, File:Line, Description)
2. 💥 **Impact & Exploitability**: (realistic risk assessment)
3. 🔧 **Remediation Steps**: (actionable recommendations for each finding)
</report>
```
