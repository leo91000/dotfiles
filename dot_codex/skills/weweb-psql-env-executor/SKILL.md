---
name: weweb-psql-env-executor
description: Execute SQL on WeWeb PostgreSQL databases by environment (`staging`, `staging-ignis`, `prod`, `local`) and service (`back`, `preview`, `plugins`). Use when a user asks to run `psql` queries, inspect data, or apply updates directly in DB, with automatic 1Password credential retrieval plus SSH or AWS SSM bastion tunneling for private remote hosts, and Docker Compose access for local.
---

# WeWeb Psql Env Executor

Run SQL on WeWeb DBs from the terminal without rebuilding connection logic each time.

## Quick Start

Run inline SQL:

```bash
 /home/leoc/.codex/skills/weweb-psql-env-executor/scripts/exec_psql_env.sh \
  --env staging \
  --service back \
  --sql 'SELECT 1;'
```

Run SQL from file:

```bash
 /home/leoc/.codex/skills/weweb-psql-env-executor/scripts/exec_psql_env.sh \
  --env staging-ignis \
  --service plugins \
  --file /tmp/query.sql
```

Pipe SQL:

```bash
cat /tmp/query.sql | /home/leoc/.codex/skills/weweb-psql-env-executor/scripts/exec_psql_env.sh --env local
```

Production read-only via AWS SSO + SSM:

```bash
/home/leoc/.codex/skills/weweb-psql-env-executor/scripts/exec_psql_env.sh \
  --env prod \
  --service back \
  --sql 'SELECT 1;'
```

## Workflow

1. Determine target `--env` and `--service` from user request.
2. Run `exec_psql_env.sh` with `--sql`, `--file`, or piped SQL.
3. Report row counts/results back to user.

## Inputs

- `--env`: `staging` | `staging-ignis` | `prod` | `production` | `local`
- `--service`: `back` | `preview` | `plugins` | `localhost`
- `--sql`: Inline SQL
- `--file`: Path to SQL file
- `--op-cache-ttl-seconds`: How long remote 1Password DB config should be cached locally, defaults to `300`. Use `0` to disable.

If `--service` is omitted:

- `staging`, `staging-ignis`, and `prod` default to `back`
- `local` defaults to `localhost`

## Notes

- The script uses 1Password item IDs already mapped for WeWeb DB entries, including prod `back`, `preview`, and `plugins`.
- Remote 1Password DB config is cached for five minutes under `/tmp/weweb-psql-env-executor/op-cache` to avoid repeated authorization prompts during multi-query work.
- For local, the script does not use 1Password. It runs `docker compose exec -T postgres psql` from `/home/leoc/projects/weweb/weweb-docker` by default.
- Local service database mapping is `back`/`localhost` -> `wwdb`, `preview` -> `wwpreview`, and `plugins` -> `wwplugins`.
- For staging envs, it creates an SSH tunnel through the bastion host.
- For prod, it keeps a shared AWS SSM tunnel open to the bastion SSH port using the `DataProd` profile and the default target `i-0221cb5e6f25851d0`.
- Prod `back`, `preview`, and `plugins` reuse that shared bastion tunnel and open a short-lived DB forward per command over the same hop.
- Prod credentials are stored directly in the `WeWeb` vault and resolved automatically by the script.
- For prod, the script auto-runs `aws sso login` when the AWS SSO session is missing.
- For repeated operations, run multiple commands in the same shell after one `op signin`.
