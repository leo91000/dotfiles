# WeWeb aliases
alias wwcd='cd ~/projects/weweb'
alias wwcde='cd ~/projects/weweb/weweb-editor'
alias wwcdd='cd ~/projects/weweb/weweb-dashboard'
alias wwcddo='cd ~/projects/weweb/weweb-docker'
alias wwcdb='cd ~/projects/weweb/weweb-docker/weweb-back'
alias wwcdbp='cd ~/projects/weweb/weweb-docker/weweb-lambda-back-publisher'
alias wwcdbs='cd ~/projects/weweb/weweb-docker/weweb-lambda-back-s3-proxy'
alias wwcdai='cd ~/projects/weweb/weweb-docker/weweb-ai'
alias wwcdp='cd ~/projects/weweb/weweb-docker/weweb-publisher'
alias wwcda='cd ~/projects/weweb/weweb-assets'

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

# Common function to handle WeWeb server startup
_ww_serve() {
  local environment="$1"
  local config="$2"
  shift 2

  local dev_mode=false
  local skip_check=false
  local mprocs_args=()

  for arg in "$@"; do
    case "$arg" in
      --dev)
        dev_mode=true
        ;;
      --skip-check|--no-check)
        skip_check=true
        ;;
      *)
        mprocs_args+=("$arg")
        ;;
    esac
  done

  local editor_script="serve"
  if [[ "$dev_mode" == true ]]; then
    if [[ "$environment" == "local" ]]; then
      editor_script="serve:dev"
    else
      editor_script="serve:dev:$environment"
    fi
    echo "Starting servers with auto-restart (DEV MODE)..."
  else
    if [[ "$environment" != "local" ]]; then
      editor_script="serve:$environment"
    fi
    echo "Starting servers with auto-restart..."
  fi

  local dashboard_script="serve"
  if [[ "$environment" != "local" ]]; then
    dashboard_script="serve:$environment"
  fi

  if [[ "$skip_check" != true ]]; then
    local projects=(
      "$HOME/projects/weweb/weweb-dashboard"
      "$HOME/projects/weweb/weweb-editor"
      $(find "$HOME/projects/weweb/weweb-docker" -maxdepth 1 -type d -exec test -d {}/.git \; -print)
    )

    echo "Starting parallel pre-checks for staging branches..."
    check_and_pull_all "${projects[@]}"
  fi

  WW_EDITOR_SCRIPT="$editor_script" WW_DASHBOARD_SCRIPT="$dashboard_script" \
    mprocs --config "$config" "${mprocs_args[@]}"
}

wws() {
  local environment="local"
  local config="$HOME/.config/mprocs/weweb-local.yaml"
  local tunnel=false
  local args=()

  for arg in "$@"; do
    case "$arg" in
      --staging)
        environment="staging"
        config="$HOME/.config/mprocs/weweb-remote.yaml"
        ;;
      --staging-ignis)
        environment="staging-ignis"
        config="$HOME/.config/mprocs/weweb-remote.yaml"
        ;;
      --preprod)
        environment="preprod"
        config="$HOME/.config/mprocs/weweb-remote.yaml"
        ;;
      --prod)
        environment="prod"
        config="$HOME/.config/mprocs/weweb-remote.yaml"
        ;;
      --tunnel)
        tunnel=true
        ;;
      *)
        args+=("$arg")
        ;;
    esac
  done

  # Use tunnel config for local environment if --tunnel is specified
  if [[ "$tunnel" == true && "$environment" == "local" ]]; then
    config="$HOME/.config/mprocs/weweb-local-tunnel.yaml"
    echo "Starting with Cloudflare tunnel to weweb-back:3000..."
  fi

  _ww_serve "$environment" "$config" "${args[@]}"
}

wwstop() {
  echo "Stopping servers..."
  cd ~/projects/weweb/weweb-docker
  docker compose down
  cd -
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
