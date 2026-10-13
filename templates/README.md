# Prompt Templates Index

This directory contains standardized, hardened prompt templates for every distinct prompt type identified in the session history.

## Prompt Types & Tag Requirements Matrix

| # | Prompt Type | File | Session Kind | `<task>` | `<acceptance_criteria>` | `<context>` | `<scope>` | `<rules>` | `<verify>` | `<report>` |
| :-: | :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| 1 | [Implementation & Refactoring](./implementation_refactoring.md) | `implementation_refactoring.md` | `write-enabled` | ✅ Req | ✅ Req | ⚪ Opt | ✅ Req | ✅ Req | ✅ Req | ✅ Req |
| 2 | [Debug & Bug Fix](./debug_bugfix.md) | `debug_bugfix.md` | `write-enabled` | ✅ Req | ✅ Req | ✅ Req | ✅ Req | ✅ Req | ✅ Req | ✅ Req |
| 3 | [Question & Consultation](./question_consultation.md) | `question_consultation.md` | `read-only` | ✅ Req | ✅ Req | ⚪ Opt | ⚪ Opt | ✅ Req | ❌ Omit | ✅ Req |
| 4 | [Plan & Architecture](./plan_architecture.md) | `plan_architecture.md` | `read-only` | ✅ Req | ✅ Req | ✅ Req | ✅ Req | ✅ Req | ✅ Req | ✅ Req |
| 5 | [Testing & Verification](./testing_verification.md) | `testing_verification.md` | `write-enabled` | ✅ Req | ✅ Req | ⚪ Opt | ✅ Req | ✅ Req | ✅ Req | ✅ Req |
| 6 | [Codebase Inspection & Audit](./codebase_audit.md) | `codebase_audit.md` | `read-only` | ✅ Req | ✅ Req | ✅ Req | ✅ Req | ✅ Req | ✅ Req | ✅ Req |
| 7 | [Report & Status Summary](./report_summary.md) | `report_summary.md` | `read-only` | ✅ Req | ✅ Req | ⚪ Opt | ✅ Req | ✅ Req | ❌ Omit | ✅ Req |
| 8 | [Research & Feasibility](./research_feasibility.md) | `research_feasibility.md` | `read-only` | ✅ Req | ✅ Req | ✅ Req | ✅ Req | ✅ Req | ⚪ Opt | ✅ Req |
| 9 | [DevOps & Deployment](./devops_deployment.md) | `devops_deployment.md` | `write-enabled` | ✅ Req | ✅ Req | ✅ Req | ✅ Req | ✅ Req | ✅ Req | ✅ Req |
| 10 | [Git & Version Control](./git_vcs.md) | `git_vcs.md` | `write-enabled` | ✅ Req | ✅ Req | ✅ Req | ✅ Req | ✅ Req | ✅ Req | ✅ Req |
| 11 | [Configuration & Tooling Setup](./config_tooling.md) | `config_tooling.md` | `write-enabled` | ✅ Req | ✅ Req | ❌ Omit | ✅ Req | ✅ Req | ✅ Req | ✅ Req |
| 12 | [Prompt Engineering & Formatting](./prompt_engineering.md) | `prompt_engineering.md` | `write-enabled` | ✅ Req | ✅ Req | ⚪ Opt | ✅ Req | ✅ Req | ✅ Req | ✅ Req |
| 13 | [Documentation & Guidelines](./documentation.md) | `documentation.md` | `write-enabled` | ✅ Req | ✅ Req | ⚪ Opt | ✅ Req | ✅ Req | ✅ Req | ✅ Req |
| 14 | [Session Progression & Approval](./session_progression.md) | `session_progression.md` | `write-enabled` | ✅ Req | ✅ Req | ✅ Req | ✅ Req | ✅ Req | ✅ Req | ✅ Req |
| 15 | [General Instruction / Task](./general_task.md) | `general_task.md` | `write-enabled` | ✅ Req | ✅ Req | ⚪ Opt | ✅ Req | ✅ Req | ✅ Req | ✅ Req |

---

## Summary of Hardcoding Strategy

1. **Anti-Hallucination & Invariants (`<rules>`)**:
   - Hardcoded per session kind (`read-only` vs `write-enabled`).
   - Read-only types (`question_consultation`, `codebase_audit`, `research_feasibility`, `report_summary`, `plan_architecture`) hardcode strict no-edit rules and citation requirements.
   - Write-enabled types hardcode minimal blast radius, preservation of tests/comments, and approval gates for destructive changes.

2. **Containment & Boundaries (`<scope> > <out_of_scope>`)**:
   - Hardcoded defaults prevent agents from wandering outside task boundaries (e.g. out-of-scope refactoring during bug fixes; making edits during audits or planning).

3. **Executable Verification (`<verify>`)**:
   - Hardcoded execution patterns guarantee that agents run tests, type checks, or linters appropriate to the operation before declaring done. Omitted for read-only consultations to save tokens.

4. **Output Format (`<report>`)**:
   - Hardcoded report schemas ensure concise, skim-friendly, predictable agent responses with standard emojis and sections.
