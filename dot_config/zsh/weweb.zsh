# WeWeb workspace and service navigation.
# wwcd: workspace; wwcd NAME: an existing checkout folder.
# Service shortcuts accept a checkout name/path, or --repo NAME_OR_PATH.
unalias wwcd wwcda wwcde wwcdd wwcddo wwcdb wwcdbp wwcdbs wwcdbai wwcdai wwcdp dcw wws wwstop wwdevcc 2>/dev/null || true
_ww_cd_service() {
  local service="$1" checkout
  shift
  if [[ "$1" == --repo ]]; then
    shift
    if (( $# != 1 )); then
      print -u2 -- 'Usage: wwcd* [CHECKOUT|--repo NAME_OR_PATH]'
      return 2
    fi
  elif (( $# > 1 )); then
    print -u2 -- 'Usage: wwcd* [CHECKOUT|--repo NAME_OR_PATH]'
    return 2
  fi
  checkout=$(_ww_monorepo_root "$1") || return
  if [[ -n "$service" ]]; then
    cd -- "$checkout/services/$service"
  else
    cd -- "$checkout"
  fi
}
wwcd() {
  if (( $# )); then
    _ww_cd_service '' "$@"
  else
    cd -- "$HOME/projects/weweb/weweb-team/weweb"
  fi
}
wwcda() {
  if (( $# > 1 )); then
    print -u2 -- 'Usage: wwcda [asset-directory]'
    return 2
  fi
  cd -- "$HOME/projects/weweb/weweb-assets/$1"
}
wwcde() { _ww_cd_service weweb-editor "$@"; }
wwcdd() { _ww_cd_service weweb-dashboard "$@"; }
wwcdb() { _ww_cd_service weweb-back "$@"; }
wwcdbp() { _ww_cd_service weweb-lambda-back-publisher "$@"; }
wwcdbs() { _ww_cd_service weweb-lambda-back-publisher/s3-proxy "$@"; }
wwcdbai() { _ww_cd_service weweb-lambda-back-publisher/ai-proxy "$@"; }
wwcdai() { _ww_cd_service weweb-ai "$@"; }
wwcdp() { _ww_cd_service weweb-lambda-publisher "$@"; }
# Legacy Docker shortcut now opens the selected monorepo checkout.
wwcddo() { _ww_cd_service '' "$@"; }
dcw() {
  local selector='' checkout
  case "$1" in
    --repo)
      if (( $# < 2 )) || [[ "$2" == --* ]]; then
        print -u2 -- 'Usage: dcw [--repo NAME_OR_PATH] [compose arguments]'
        return 2
      fi
      selector="$2"
      shift 2
      ;;
  esac
  checkout=$(_ww_monorepo_root "$selector") || return
  (cd -- "$checkout" && docker compose "$@")
}

# Function to check for `staging` branch and pull changes
check_and_pull() {
  local project=$1
  echo "Checking project: $project"

  if [ -d "$project/.git" ]; then
    cd "$project" || return

    # Check for local changes
    if [ -z "$(git status --porcelain -uno)" ]; then
      echo "No local changes detected on staging branch in $project. Pulling latest changes..."
      git pull
    else
      echo "Local changes detected in $project. Skipping pull."
    fi

    cd - > /dev/null || return
  else
    echo "$project is not a git repository. Skipping."
  fi
}

check_and_pull_all() {
  local projects=("$@")
  local names=()
  local concurrently_commands=()
  local weweb_zsh="$HOME/.config/zsh/weweb.zsh"

  for project in "${projects[@]}"; do
    # Create the command for this project
    local name=$(basename "$project")
    names+=("$name")
    # Source weweb helpers first, then run our command
    concurrently_commands+=("zsh -c 'source \"$weweb_zsh\" && cd \"$project\" && check_and_pull .'" )
  done

  # Join arrays with commas for the names
  local names_str=$(IFS=,; echo "${names[*]}")

  echo "Executing parallel checks..."
  npx -y concurrently --names "$names_str" "${concurrently_commands[@]}"
}

# Resolve a monorepo checkout without changing its branch.
_ww_monorepo_root() {
  local checkout="$1" base="$HOME/projects/weweb/weweb-team/weweb"
  if [[ -z "$checkout" ]]; then
    checkout="${WW_MONOREPO_ROOT:-}"
    if [[ -z "$checkout" ]]; then
      local current_root
      current_root=$(git rev-parse --show-toplevel 2>/dev/null)
      if [[ -f "$current_root/services/weweb-editor/package.json" && -f "$current_root/docker-compose.yml" ]]; then
        checkout="$current_root"
      else
        checkout="$base/main"
      fi
    fi
  fi
  if [[ "$checkout" == --* ]]; then
    print -u2 -- 'Use a checkout folder name or --repo NAME_OR_PATH; feature-specific flags are not supported.'
    return 2
  fi
  # Resolve bare names at call time; no feature branches are encoded here.
  if [[ "$checkout" != */* ]]; then
    checkout="$base/$checkout"
  fi
  if [[ ! -f "$checkout/services/weweb-editor/package.json" || ! -f "$checkout/docker-compose.yml" ]]; then
    print -u2 -- "Not a WeWeb monorepo checkout: $checkout"
    return 1
  fi
  print -r -- "${checkout:A}"
}

# Common function to handle WeWeb server startup.
_ww_serve() {
  local environment="$1" config="$2" checkout="$3"
  shift 3
  local dev_mode='' skip_check=false arg
  local mprocs_args=()
  for arg in "$@"; do
    case "$arg" in
      --dev) dev_mode=true ;;
      --skip-check|--no-check) skip_check=true ;;
      *) mprocs_args+=("$arg") ;;
    esac
  done

  local editor_mode=development dashboard_mode=development
  if [[ "$environment" == prod ]]; then
    editor_mode=prod
    dashboard_mode=production
  elif [[ "$environment" != local ]]; then
    editor_mode="development.$environment"
    dashboard_mode="local.$environment"
  fi

  local node_root
  node_root=$(mise where node@24) || return
  if [[ ! -x "$node_root/bin/node" ]]; then
    print -u2 -- 'Node 24 is required. Install it with: mise install node@24'
    return 1
  fi

  if [[ "$skip_check" != true ]]; then
    if [[ -n "$(git -C "$checkout" status --porcelain)" ]]; then
      print -- 'Local changes detected; skipping pull.'
    elif git -C "$checkout" rev-parse --verify '@{upstream}' >/dev/null 2>&1; then
      git -C "$checkout" pull --ff-only || return
    else
      print -- 'No upstream configured; skipping pull.'
    fi
  fi

  print -- "Starting WeWeb ($environment) from $checkout"
  WW_MONOREPO_ROOT="$checkout" WW_EDITOR_MODE="$editor_mode" \
    WW_DASHBOARD_MODE="$dashboard_mode" WW_EDITOR_DEV="$dev_mode" \
    PATH="$node_root/bin:$PATH" mprocs --config "$config" "${mprocs_args[@]}"
}

wws() {
  local environment=local config="$HOME/.config/mprocs/weweb-local.yaml"
  local tunnel=false checkout='' arg
  local args=()
  while (( $# )); do
    arg="$1"
    shift
    case "$arg" in
      --staging|--staging-ignis|--preprod|--prod|--beta)
        environment="${arg#--}"
        config="$HOME/.config/mprocs/weweb-remote.yaml"
        ;;
      --tunnel) tunnel=true ;;
      --repo)
        if (( ! $# )) || [[ "$1" == --* ]]; then
          print -u2 -- 'Usage: wws --repo NAME_OR_PATH'
          return 2
        fi
        checkout="$1"
        shift
        ;;
      --help|-h)
        print -- 'Usage: wws [--repo NAME_OR_PATH] [--dev] [--skip-check] [--staging|--staging-ignis|--preprod|--prod|--beta] [--tunnel] [mprocs options]'
        print -- 'Checkout: explicit name/path, WW_MONOREPO_ROOT, current monorepo, then main.'
        return
        ;;
      *) args+=("$arg") ;;
    esac
  done
  checkout=$(_ww_monorepo_root "$checkout") || return
  if [[ "$tunnel" == true && "$environment" == local ]]; then
    config="$HOME/.config/mprocs/weweb-local-tunnel.yaml"
    print -- 'Starting with Cloudflare tunnel to weweb-back:3000...'
  fi
  _ww_serve "$environment" "$config" "$checkout" "${args[@]}"
}

wwstop() {
  local checkout
  if [[ "$1" == --repo ]]; then
    shift
    if (( $# != 1 )); then
      print -u2 -- 'Usage: wwstop [CHECKOUT|--repo NAME_OR_PATH]'
      return 2
    fi
  elif (( $# > 1 )); then
    print -u2 -- 'Usage: wwstop [CHECKOUT|--repo NAME_OR_PATH]'
    return 2
  fi
  checkout=$(_ww_monorepo_root "$1") || return
  print -- "Stopping monorepo Docker services from $checkout..."
  (cd "$checkout" && docker compose stop)
}

function wwdevcc() {
  local commands=()
  local names=()
  local colors=()
  local ports=()
  local cert_paths=()
  local params=()
  local i=0

  for param in "$@"
  do
    local port=$((8080 + i))
    ports+=("$port")
    params+=("$param")
    local cmd="cd ~/projects/weweb/weweb-assets/ww-${param} && npm i && npm run serve port=${port}"
    commands+=("$cmd")
    names+=("ww:cc:${param}")

    local color_list=("red" "green" "yellow" "blue" "magenta" "cyan" "white" "gray")
    local color="${color_list[$i % ${#color_list[@]}]}"
    colors+=("$color")

    # Store the certificate path for each parameter
    local cert_path="$HOME/projects/weweb/weweb-assets/ww-${param}/node_modules/@weweb/cli/node_modules/.cache/webpack-dev-server/server.pem"
    cert_paths+=("$cert_path")

    ((i++))
  done

  # Join the names and colors into comma-separated strings
  local names_str="${(j:,:)names}"
  local colors_str="${(j:,:)colors}"

  # Kill all processes listening on the ports
  for port in "${ports[@]}"
  do
    # Find and kill processes listening on the port
    lsof -ti tcp:"$port" | xargs -r kill -9
  done

  # Run the commands concurrently with npx and capture the PID
  npx --yes concurrently --names "$names_str" --prefix-colors "$colors_str" "${commands[@]}" &
  local concurrently_pid=$!

  # Give the servers some time to start and generate certificates
  sleep 5

  # Import certificates into Chrome's NSS database
  echo "Importing certificates into Chrome's NSS database..."
  local index=1
  for cert_path in "${cert_paths[@]}"
  do
    local param="${params[$index]}"

    # Sanitize the nickname by replacing special characters
    local sanitized_param="${param//[: ]/_}"

    # Check if the certificate file exists
    if [[ -f "$cert_path" ]]; then
      # Convert PEM to DER format
      local der_cert="/tmp/ww-${sanitized_param}-cert.der"
      openssl x509 -in "$cert_path" -outform der -out "$der_cert"

      # Generate a unique nickname using the sanitized parameter name
      local cert_nickname="ww-${sanitized_param}-cert"

      # Delete any existing certificate with the same nickname to avoid conflicts
      certutil -d sql:"$HOME/.pki/nssdb" -D -n "$cert_nickname" 2>/dev/null

      # Import the certificate into Chrome's NSS database
      certutil -d sql:"$HOME/.pki/nssdb" -A -t "P,," -n "$cert_nickname" -i "$der_cert"

      # Clean up the DER certificate
      rm "$der_cert"

      echo "Imported $cert_nickname into Chrome's NSS database."
    else
      echo "Certificate not found at $cert_path"
    fi

    ((index++))
  done

  # Wait for the concurrently process to finish (optional)
  wait $concurrently_pid
}
