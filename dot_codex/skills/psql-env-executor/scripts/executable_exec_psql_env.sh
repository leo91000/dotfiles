#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  exec_psql_env.sh --env <staging|staging-ignis|local> [--service <back|preview|plugins|localhost>] [--sql <SQL> | --file <path>]
  exec_psql_env.sh --env <staging|staging-ignis|local> [--service <back|preview|plugins|localhost>] < input.sql

Options:
  --env           Target environment
  --service       Target service
  --sql           SQL statement(s)
  --file          SQL file path
  --op-account    1Password account shorthand (default: my.1password.eu)
  --local-port    Local forwarded port for SSH tunnel (default: 15432)
  --help          Show this help
USAGE
}

ENV_NAME=""
SERVICE=""
SQL_TEXT=""
SQL_FILE=""
OP_ACCOUNT_VALUE="${OP_ACCOUNT:-my.1password.eu}"
LOCAL_PORT="15432"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --env)
      ENV_NAME="${2:-}"
      shift 2
      ;;
    --service)
      SERVICE="${2:-}"
      shift 2
      ;;
    --sql)
      SQL_TEXT="${2:-}"
      shift 2
      ;;
    --file)
      SQL_FILE="${2:-}"
      shift 2
      ;;
    --op-account)
      OP_ACCOUNT_VALUE="${2:-}"
      shift 2
      ;;
    --local-port)
      LOCAL_PORT="${2:-}"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ -z "$ENV_NAME" ]]; then
  echo "Missing --env" >&2
  usage >&2
  exit 1
fi

if [[ -n "$SQL_TEXT" && -n "$SQL_FILE" ]]; then
  echo "Use only one of --sql or --file" >&2
  exit 1
fi

if [[ -z "$SERVICE" ]]; then
  case "$ENV_NAME" in
    staging|staging-ignis)
      SERVICE="back"
      ;;
    local)
      SERVICE="localhost"
      ;;
    *)
      echo "Unsupported env: $ENV_NAME" >&2
      exit 1
      ;;
  esac
fi

KEY="${ENV_NAME}:${SERVICE}"
ITEM_ID=""
VAULT="WeWeb"
BASTION_HOST="3.224.157.48"
BASTION_USER="ec2-user"

case "$KEY" in
  staging:back) ITEM_ID="k7i2nla3wxsvy7zmn5buqouan4" ;;
  staging:preview) ITEM_ID="n3ntvnicjwg3np2hyapqfnh57a" ;;
  staging:plugins) ITEM_ID="7ujhrwvjz6tee5ld6swvj7w4nm" ;;
  staging-ignis:back) ITEM_ID="yoyuvrpxyh66xgsgi5voawstpm" ;;
  staging-ignis:preview) ITEM_ID="yuzyrkgv6yovwa4yoz6y6gr76a" ;;
  staging-ignis:plugins) ITEM_ID="dn6zxuytzn4yqhrd2fvw23hjm4" ;;
  local:localhost|local:back|local:preview|local:plugins) ITEM_ID="exzkwrhkiax4mcjpxo3b7t4sqe" ;;
  *)
    echo "Unsupported env/service combination: $KEY" >&2
    exit 1
    ;;
esac

if ! op whoami >/dev/null 2>&1; then
  op signin --account "$OP_ACCOUNT_VALUE" >/dev/null
fi

ITEM_JSON="$(op item get "$ITEM_ID" --vault "$VAULT" --format json)"
DB_HOST="$(printf '%s' "$ITEM_JSON" | jq -r '.fields[] | select(.label=="server") | .value // empty')"
DB_PORT="$(printf '%s' "$ITEM_JSON" | jq -r '.fields[] | select(.label=="port") | .value // "5432"')"
DB_NAME="$(printf '%s' "$ITEM_JSON" | jq -r '.fields[] | select(.label=="database") | .value // empty')"
DB_USER="$(printf '%s' "$ITEM_JSON" | jq -r '.fields[] | select(.label=="username") | .value // empty')"
DB_PASS="$(printf '%s' "$ITEM_JSON" | jq -r '.fields[] | select(.label=="password") | .value // empty')"
SSH_ENABLED="$(printf '%s' "$ITEM_JSON" | jq -r '.fields[] | select(.label=="ssh_enabled") | .value // "false"')"
SSH_KEY="$(printf '%s' "$ITEM_JSON" | jq -r '.fields[] | select(.label=="ssh_key") | .value // empty')"

if [[ -z "$DB_HOST" || -z "$DB_NAME" || -z "$DB_USER" || -z "$DB_PASS" ]]; then
  echo "Missing required DB credentials in item: $ITEM_ID" >&2
  exit 1
fi

SQL_PAYLOAD=""
if [[ -n "$SQL_TEXT" ]]; then
  SQL_PAYLOAD="$SQL_TEXT"
elif [[ -n "$SQL_FILE" ]]; then
  if [[ ! -f "$SQL_FILE" ]]; then
    echo "SQL file not found: $SQL_FILE" >&2
    exit 1
  fi
  SQL_PAYLOAD="$(cat "$SQL_FILE")"
else
  if [[ -t 0 ]]; then
    echo "Provide --sql, --file, or stdin SQL" >&2
    exit 1
  fi
  SQL_PAYLOAD="$(cat)"
fi

if [[ -z "$SQL_PAYLOAD" ]]; then
  echo "Empty SQL input" >&2
  exit 1
fi

KEY_FILE=""
TUNNEL_PID=""
cleanup() {
  if [[ -n "$TUNNEL_PID" ]] && kill -0 "$TUNNEL_PID" 2>/dev/null; then
    kill "$TUNNEL_PID" >/dev/null 2>&1 || true
    wait "$TUNNEL_PID" 2>/dev/null || true
  fi
  if [[ -n "$KEY_FILE" && -f "$KEY_FILE" ]]; then
    rm -f "$KEY_FILE"
  fi
}
trap cleanup EXIT

if [[ "$ENV_NAME" != "local" && "$SSH_ENABLED" == "true" ]]; then
  if [[ -z "$SSH_KEY" ]]; then
    echo "ssh_enabled=true but ssh_key is missing in item: $ITEM_ID" >&2
    exit 1
  fi
  KEY_FILE="$(mktemp)"
  printf '%s\n' "$SSH_KEY" | sed '1s/^"//; $s/"$//' > "$KEY_FILE"
  chmod 600 "$KEY_FILE"

  ssh -o BatchMode=yes -o ExitOnForwardFailure=yes -o StrictHostKeyChecking=no -o UserKnownHostsFile=/tmp/weweb_known_hosts -i "$KEY_FILE" -N -L "${LOCAL_PORT}:${DB_HOST}:${DB_PORT}" "${BASTION_USER}@${BASTION_HOST}" >/tmp/psql-env-executor-ssh.log 2>&1 &
  TUNNEL_PID="$!"
  sleep 2

  if ! kill -0 "$TUNNEL_PID" 2>/dev/null; then
    echo "Failed to start SSH tunnel" >&2
    cat /tmp/psql-env-executor-ssh.log >&2 || true
    exit 1
  fi

  DB_HOST="127.0.0.1"
  DB_PORT="$LOCAL_PORT"
fi

export PGPASSWORD="$DB_PASS"
printf '%s\n' "$SQL_PAYLOAD" | psql "host=${DB_HOST} port=${DB_PORT} dbname=${DB_NAME} user=${DB_USER} sslmode=require connect_timeout=10" -v ON_ERROR_STOP=1 -P pager=off
