# Template: DevOps & Deployment

- **File**: `templates/devops_deployment.md`
- **Session Kind**: `write-enabled`
- **Primary Intent**: Managing container configurations, Dockerfiles, Nginx configs, CI/CD workflows, server scripts, and worker deployments.

---

## 1. Needed Parts (Tags Analysis)

| Tag | Status | Mode | Rationale |
| :--- | :---: | :---: | :--- |
| `<task>` | **Required** | Dynamic | Server configuration, Docker container build, deployment, or infrastructure script. |
| `<acceptance_criteria>` | **Required** | Dynamic | Config syntax valid; containers healthy; zero downtime; secret isolation. |
| `<context>` | **Required** | Dynamic | Environment (staging/prod), endpoint hosts, port mappings, secret keys names. |
| `<scope>` | **Required** | Mixed | Infra files dynamic; hardcode out-of-scope for plaintext secrets and direct DB data mutation. |
| `<rules>` | **Required** | Hardcoded | Never log or commit plaintext secrets, dry-run configs, idempotent operations, clear rollback. |
| `<verify>` | **Required** | Hardcoded Pattern | Syntax check (`docker compose config`, `nginx -t`), container build, endpoint health check. |
| `<report>` | **Required** | Hardcoded | Infrastructure Changes, Security Check, Verification & Rollback. |

---

## 2. Hardcoded vs Dynamic Data Analysis

### Static / Hardcoded Defaults
#### `scope > out_of_scope`
```text
Committing plaintext secrets, mutating production database data directly, downtime-causing actions without backup.
```

#### `rules`
```text
- NEVER commit, display, or log plaintext secrets, private keys, or credentials.
- Dry-run and syntax check configurations before applying (`docker compose config`, `nginx -t`).
- All operational scripts must be idempotent (safe to run repeatedly).
- Always provide an immediate rollback procedure.
- Be EXTREMELY concise.
```

#### `verify`
```text
- Run configuration syntax checks (`docker compose config`, `nginx -t`, etc.).
- Verify container builds cleanly without caching errors.
- Test health check endpoint or daemon status.
```

#### `report`
```text
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 🚀 **Infrastructure Changes**: (containers, configs, scripts updated)
2. 🔒 **Security & Secrets Check**: (confirmation no secrets leaked)
3. 🔄 **Verification & Rollback**: (health check output and rollback command)
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
Configure / deploy infrastructure for: {{infrastructure_or_service_target}}
</task>

<acceptance_criteria>
- Configuration syntax passes validation cleanly
- Services start successfully and pass health checks
- No credentials or secrets exposed in code, logs, or history
- Rollback command or recovery procedure documented
</acceptance_criteria>

<context>
- Target environment: {{environment_staging_or_prod}}
- Port & network mappings: {{ports_and_networks}}
</context>

<scope>
<relevant_files>{{infra_files_e.g._Dockerfile_compose_conf}}</relevant_files>
<out_of_scope>Committing plaintext secrets, mutating production database data directly</out_of_scope>
</scope>

<rules>
- NEVER commit, display, or log plaintext secrets, private keys, or credentials.
- Dry-run and syntax check configurations before applying (`docker compose config`, `nginx -t`).
- All operational scripts must be idempotent (safe to run repeatedly).
- Always provide an immediate rollback procedure.
- Be EXTREMELY concise.
</rules>

<verify>
- Run configuration syntax checks (`docker compose config`, `nginx -t`, etc.)
- Test service build and check container health: `{{health_check_command}}`
- Verify no plaintext secrets present in git diff: `git diff | grep -iE 'key|secret|password|token'`
</verify>

<report>
KEEP YOUR RESPONSE EXTREMELY SHORT.
1. 🚀 **Infrastructure Changes**: (containers, configs, scripts updated)
2. 🔒 **Security & Secrets Check**: (confirmation no secrets leaked)
3. 🔄 **Verification & Rollback**: (health check output and rollback command)
</report>
```
