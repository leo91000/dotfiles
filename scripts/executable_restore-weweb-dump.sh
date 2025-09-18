#!/usr/bin/env zsh

# Colors for pretty output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default setting for skipping database restore
SKIP_DB_RESTORE=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-db-restore)
            SKIP_DB_RESTORE=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--skip-db-restore]"
            exit 1
            ;;
    esac
done

# Function to print step information
print_step() {
    echo -e "${BLUE}==>${NC} ${GREEN}$1${NC}"
}

# Function to print error and exit
print_error_and_exit() {
    echo -e "${RED}ERROR: $1${NC}"
    exit 1
}

# Function to check if a command exists
check_command() {
    if ! command -v $1 &> /dev/null; then
        print_error_and_exit "$1 could not be found. Please install it and try again."
    fi
}

# Check for required commands
check_command docker
check_command pg_restore
check_command psql

# Path configurations (allow override via env)
# You can run with: DUMPS_DIR=/path/to/dumps WEWEB_DOCKER_DIR=/path/to/weweb-docker ./restore-weweb-dump.sh
DUMPS_DIR="${DUMPS_DIR:-$HOME/Téléchargements}"
# If Downloads exists and Téléchargements does not, prefer Downloads (macOS label vs folder name)
if [ ! -d "$DUMPS_DIR" ]; then
    if [ -d "$HOME/Downloads" ]; then
        DUMPS_DIR="$HOME/Downloads"
    fi
fi
WEWEB_DOCKER_DIR="${WEWEB_DOCKER_DIR:-$HOME/projects/weweb/weweb-docker}"
DOCKER_COMPOSE_FILE="$WEWEB_DOCKER_DIR/docker-compose.yml"

# Check if docker-compose file exists
if [ ! -f "$DOCKER_COMPOSE_FILE" ]; then
    print_error_and_exit "docker-compose.yml not found at $DOCKER_COMPOSE_FILE"
fi

# Check if dump files exist
print_step "Checking for dump files in $DUMPS_DIR"
BACK_DUMP="$DUMPS_DIR/back.dump"
PREVIEW_DUMP="$DUMPS_DIR/preview.dump"
PLUGINS_DUMP="$DUMPS_DIR/plugin.dump"

if [ ! -f "$BACK_DUMP" ] || [ ! -f "$PREVIEW_DUMP" ] || [ ! -f "$PLUGINS_DUMP" ]; then
    print_error_and_exit "Missing dump files in $DUMPS_DIR. Expected: back.dump, preview.dump, plugin.dump"
fi

echo "Found dump files:"
echo "- BACK: $(basename "$BACK_DUMP")"
echo "- PREVIEW: $(basename "$PREVIEW_DUMP")"
echo "- PLUGINS: $(basename "$PLUGINS_DUMP")"

# Start PostgreSQL using docker-compose
print_step "Starting PostgreSQL container"
cd "$WEWEB_DOCKER_DIR" || print_error_and_exit "Could not change to directory $WEWEB_DOCKER_DIR"
docker compose up -d postgres || print_error_and_exit "Failed to start PostgreSQL container"

# Wait for PostgreSQL to be ready
print_step "Waiting for PostgreSQL to be ready"
for i in {1..30}; do
    if docker compose exec postgres pg_isready -h localhost -U wwdb; then
        echo "PostgreSQL is ready!"
        break
    fi
    if [ $i -eq 30 ]; then
        print_error_and_exit "PostgreSQL is not ready after 30 seconds. Please check the container."
    fi
    echo "Waiting for PostgreSQL to be ready... ($i/30)"
    sleep 1
done

# Function to reset/create a database
reset_database() {
    local db_name=$1
    echo -e "${YELLOW}Resetting/Creating database: $db_name${NC}"
    docker compose exec postgres psql -U postgres -c "DROP DATABASE IF EXISTS $db_name;"
    docker compose exec postgres psql -U postgres -c "CREATE DATABASE $db_name WITH OWNER wwdb;"
}

# Function to restore a dump
restore_dump() {
    local db_name=$1
    local dump_file=$2
    local dump_basename=$(basename "$dump_file")
    
    echo -e "${YELLOW}Restoring $dump_basename to $db_name${NC}"
    echo "This may take several minutes, especially for wwdb..."
    
    # Using docker cp to copy the dump file into the container and then restore
    # This avoids path issues between host and container
    docker compose exec postgres mkdir -p /tmp/dumps
    docker cp "$dump_file" "$(docker compose ps -q postgres):/tmp/dumps/$dump_basename"
    docker compose exec postgres pg_restore -U wwdb -d "$db_name" -v "/tmp/dumps/$dump_basename"
    
    if [ $? -ne 0 ]; then
        echo -e "${YELLOW}Warning: pg_restore completed with some errors, but this might be acceptable${NC}"
    else
        echo -e "${GREEN}Successfully restored $dump_basename to $db_name${NC}"
    fi
}

# Reset/Create and restore databases if not skipping
if [ "$SKIP_DB_RESTORE" = false ]; then
    # Reset/Create databases
    print_step "Resetting/Creating databases"
    reset_database "wwdb"
    reset_database "wwpreview"
    reset_database "wwplugins"
    
    # Restore dumps
    print_step "Restoring database dumps (this will take time)"
    restore_dump "wwdb" "$BACK_DUMP"
    restore_dump "wwpreview" "$PREVIEW_DUMP"
    restore_dump "wwplugins" "$PLUGINS_DUMP"
else
    print_step "Skipping database reset and restoration as requested"
fi

# Start all services with docker compose
print_step "Starting all WeWeb services with docker compose"
docker compose up -d || print_error_and_exit "Failed to start WeWeb services"

# Enable UUID extension for the databases (regardless of whether we restored them)
print_step "Enabling UUID extension for databases"
docker compose exec postgres psql -U postgres -d "wwdb" -c "CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";"
docker compose exec postgres psql -U postgres -d "wwpreview" -c "CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";"
docker compose exec postgres psql -U postgres -d "wwplugins" -c "CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";"

# Prompt user for email
print_step "Please enter your WeWeb email address:"
read USER_EMAIL

if [ -z "$USER_EMAIL" ]; then
    print_error_and_exit "No email provided. Exiting."
fi

# Find user ID by email
print_step "Finding user ID by email"
USER_ID=$(docker compose exec postgres psql -U wwdb -d wwdb -t -c "SELECT id FROM \"Users\" WHERE email = '$USER_EMAIL';" | tr -d '[:space:]')

if [ -z "$USER_ID" ]; then
    print_error_and_exit "User not found with email $USER_EMAIL. Please check that you've logged in successfully."
fi

echo "Found user ID: $USER_ID"

# Insert user into UserOrganizations
print_step "Adding user to WeWeb organization"
docker compose exec postgres psql -U wwdb -d wwdb -c "INSERT INTO \"UserOrganizations\" (id, \"userId\", \"organizationId\", \"role\", \"createdAt\", \"updatedAt\") VALUES (uuid_generate_v4(), '$USER_ID', '3f66defb-254f-4495-99dd-d1ecb6960259', 'admin', NOW(), NOW()) ON CONFLICT DO NOTHING;"

# Make user an admin
print_step "Making user an admin"
docker compose exec postgres psql -U wwdb -d wwdb -c "INSERT INTO \"AdminUsers\" (id, \"userId\", \"createdAt\", \"updatedAt\") VALUES (uuid_generate_v4(), '$USER_ID', NOW(), NOW()) ON CONFLICT DO NOTHING;"

echo -e "\n${GREEN}Setup complete!${NC}"
echo -e "${YELLOW}Important next steps:${NC}"
echo "1. In the front-end, click on the WeWeb workspace"
echo "2. Go to Coded Components tab"
echo "3. First download all plugins (Set all to latest + Fetch all)"
echo "4. Then download all sections (Set all to latest + Fetch all)"
echo "5. Finally download all elements (Set all to latest + Fetch all)"
echo "6. Do a final 'Set all to latest version' on all 3 tabs"

echo -e "\n${YELLOW}WeWeb services are running in the background${NC}"
echo "You can stop them anytime by running 'docker compose down' in $WEWEB_DOCKER_DIR"

exit 0
