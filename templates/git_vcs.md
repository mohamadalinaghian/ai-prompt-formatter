# Template: Git & Version Control

- **File**: `templates/git_vcs.md`
- **Session Kind**: `write-enabled`
- **Primary Intent**: Authoring conventional commits, managing branches, opening pull requests, and resolving merge conflicts.

---

## 1. Needed Parts (Tags Analysis)

| Tag | Status | Mode | Rationale |
| :--- | :---: | :---: | :--- |
| `<task>` | **Required** | Dynamic | Specific git operation (commit, branch, PR, rebase, merge). |
| `<acceptance_criteria>` | **Required** | Dynamic | Clean git status, Conventional Commit format, no untracked artifact commits. |
| `<context>` | **Required** | Dynamic | Current branch, git status output, staged files, PR or issue reference. |
| `<scope>` | **Required** | Mixed | Target staged files dynamic; secret files, .env, and build artifacts out of scope. |
| `<rules>` | **Required** | Hardcoded | Conventional commits, never force-push to main, no untracked secrets, verify diff before commit. |
| `<verify>` | **Required** | Hardcoded Pattern | `git status` clean, `git log -1` conforms to conventional commit formatting. |
| `<report>` | **Required** | Hardcoded | Git Operation, Commit Message/Summary, Cleanliness Check. |

---

## 2. Hardcoded vs Dynamic Data Analysis

### Static / Hardcoded Defaults
#### `scope > out_of_scope`
```text
Committing untracked secrets, `.env` files, build artifacts (`target/`, `node_modules/`), force-pushing to main.
```

#### `rules`
```text
- Follow Conventional Commits format (`feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`).
- Never force-push (`git push --force`) to main or protected branches.
- Never commit untracked files without explicit verification.
- Verify `git status` and staged diff before running commit.
- Be EXTREMELY concise.
```

#### `verify`
```text
- Verify working tree status: `git status`.
- Verify latest commit formatting: `git log -1 --stat`.
```

#### `report`
```text
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 📦 **Git Operation**: (branch name, commit hash, or PR status)
2. 📝 **Commit Message / Summary**: (title and bullet points)
3. 🔍 **Cleanliness Check**: (verification that no dirty/untracked files remain)
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
Perform git operation: {{git_task_description_e.g._commit_changes_or_create_pr}}
</task>

<acceptance_criteria>
- Commit message follows Conventional Commits format
- Only relevant, intended files are staged and committed
- Working tree remains clean after operation
- No secrets or binary build artifacts committed
</acceptance_criteria>

<context>
- Target branch: {{target_branch}}
- Summary of changes: {{summary_of_changes}}
</context>

<scope>
<relevant_files>{{staged_files}}</relevant_files>
<out_of_scope>Committing untracked secrets, .env files, build artifacts (target/, node_modules/), force-pushing to main</out_of_scope>
</scope>

<rules>
- Follow Conventional Commits format (`feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`).
- Never force-push (`git push --force`) to main or protected branches.
- Never commit untracked files without explicit verification.
- Verify `git status` and staged diff before running commit.
- Be EXTREMELY concise.
</rules>

<verify>
- Verify working tree status: `git status`
- Verify latest commit formatting: `git log -1 --stat`
</verify>

<report>
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 📦 **Git Operation**: (branch name, commit hash, or PR status)
2. 📝 **Commit Message / Summary**: (title and bullet points)
3. 🔍 **Cleanliness Check**: (verification that no dirty/untracked files remain)
</report>
```
