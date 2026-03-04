---
name: weweb-psql-env-executor
description: Execute SQL on WeWeb PostgreSQL databases by environment (`staging`, `staging-ignis`, `local`) and service (`back`, `preview`, `plugins`). Use when a user asks to run `psql` queries, inspect data, or apply updates directly in DB, with automatic 1Password credential retrieval and SSH tunneling for private staging RDS hosts.
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

## Workflow

1. Determine target `--env` and `--service` from user request.
2. Run `exec_psql_env.sh` with `--sql`, `--file`, or piped SQL.
3. Report row counts/results back to user.

## Inputs

- `--env`: `staging` | `staging-ignis` | `local`
- `--service`: `back` | `preview` | `plugins` | `localhost`
- `--sql`: Inline SQL
- `--file`: Path to SQL file

If `--service` is omitted:

- `staging` and `staging-ignis` default to `back`
- `local` defaults to `localhost`

## Notes

- The script uses 1Password item IDs already mapped for WeWeb DB entries.
- For staging envs, it creates an SSH tunnel through the bastion host.
- For repeated operations, run multiple commands in the same shell after one `op signin`.
