---
name: weweb-psql-env-executor
description: Execute SQL on WeWeb PostgreSQL databases by environment (`beta`, `preprod`, `staging`, `staging-ignis`, `prod`, `local`) and service (`back`, `preview`, `plugins`). Use when a user asks to run `psql` queries, inspect data, or apply updates directly in DB, with automatic AWS SSM or 1Password credential retrieval plus tunneling for private remote hosts, and Docker Compose access for local.
---

# WeWeb Psql Env Executor

Run SQL on WeWeb DBs from the terminal without rebuilding connection logic each time.

## Quick Start

Run inline SQL:

```bash
 $HOME/.agents/skills/weweb-psql-env-executor/scripts/exec_psql_env.sh \
  --env staging \
  --service back \
  --sql 'SELECT 1;'
```

Run read-only SQL against preprod when the AWS profile has SSM session access:

```bash
$HOME/.agents/skills/weweb-psql-env-executor/scripts/exec_psql_env.sh \
  --env preprod \
  --service back \
  --sql 'SELECT 1;'
```

Run SQL against a local stack with an auto-published Postgres port:

```bash
 $HOME/.agents/skills/weweb-psql-env-executor/scripts/exec_psql_env.sh \
  --env local \
  --service back \
  --postgres-port 4917 \
  --sql 'SELECT 1;'
```

Run SQL from file:

```bash
 $HOME/.agents/skills/weweb-psql-env-executor/scripts/exec_psql_env.sh \
  --env staging-ignis \
  --service plugins \
  --file /tmp/query.sql
```

Pipe SQL:

```bash
cat /tmp/query.sql | $HOME/.agents/skills/weweb-psql-env-executor/scripts/exec_psql_env.sh --env local
```

Production read-only via AWS SSO + SSM:

```bash
$HOME/.agents/skills/weweb-psql-env-executor/scripts/exec_psql_env.sh \
  --env prod \
  --service back \
  --sql 'SELECT 1;'
```

## Workflow

1. Determine target `--env` and `--service` from user request.
2. Run `exec_psql_env.sh` with `--sql`, `--file`, or piped SQL.
3. Report row counts/results back to user.

## Inputs

- `--env`: `beta` | `preprod` | `staging` | `staging-ignis` | `prod` | `production` | `local`
- `--service`: `back` | `preview` | `plugins` | `localhost`
- `--sql`: Inline SQL
- `--file`: Path to SQL file
- `--postgres-port`: For `--env local` only, connect directly to `127.0.0.1:<port>` instead of using Docker Compose exec. Use this when the local stack publishes Postgres on a dynamic host port.
- `--ssh-key`: Private key path for the preprod or production bastion SSH hop. Preprod defaults to `~/.ssh/id_ed25519`; production defaults to the key stored in 1Password.
- `--op-cache-ttl-seconds`: How long remote 1Password DB config should be cached locally, defaults to `300`. Use `0` to disable.

If `--service` is omitted:

- `beta`, `preprod`, `staging`, `staging-ignis`, and `prod` default to `back`
- `local` defaults to `localhost`

## Notes

- Beta database credentials are read from encrypted AWS SSM parameters with the `weweb-beta` read-only profile, while the private network tunnel uses `DataBeta`; the credentials are not stored in 1Password.
- Preprod database credentials are read from the mapped `WeWeb` vault items in 1Password. The private network tunnel uses `DataPreprod` to forward the bastion SSH port, then `~/.ssh/id_ed25519` by default to forward PostgreSQL through the bastion.
- The script uses 1Password item IDs already mapped for staging, preprod, and production `back`, `preview`, and `plugins` databases.
- Remote 1Password DB config is cached for five minutes under `/tmp/weweb-psql-env-executor/op-cache` to avoid repeated authorization prompts during multi-query work.
- For local, the script does not use 1Password. It runs `docker compose exec -T postgres psql` from `/home/leoc/projects/weweb/weweb-docker` by default. Pass `--postgres-port` to use a local published Postgres port instead.
- Local service database mapping is `back`/`localhost` -> `wwdb`, `preview` -> `wwpreview`, and `plugins` -> `wwplugins`.
- For staging AWS envs, it creates an AWS SSM `AWS-StartPortForwardingSessionToRemoteHost` tunnel from the bastion instance directly to the database host.
- For preprod and prod, it keeps the compatible AWS SSM tunnel to the bastion SSH port, then opens a short-lived SSH DB forward over that local hop.
- Bastion instances are discovered from EC2 tags, with `--ssm-target` available for manual override.
- Prod credentials are stored directly in the `WeWeb` vault and resolved automatically by the script.
- For AWS profiles, the script auto-runs `aws sso login` when the AWS SSO session is missing.
- For repeated operations, run multiple commands in the same shell after one `op signin`.
