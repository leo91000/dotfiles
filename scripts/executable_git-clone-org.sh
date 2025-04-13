#!/bin/bash

# Script to clone all repositories from a GitHub organization using GitHub CLI
# Usage: ./script.sh <organization-name>

# Default organization if none provided
ORG=${1:-"weweb-assets"}

# Ensure GitHub CLI is installed
if ! command -v gh &> /dev/null; then
    echo "GitHub CLI (gh) is not installed. Please install it first:"
    echo "  https://cli.github.com/manual/installation"
    exit 1
fi

# Ensure user is authenticated with GitHub CLI
if ! gh auth status &> /dev/null; then
    echo "You're not authenticated with GitHub CLI. Please run 'gh auth login' first."
    exit 1
fi

echo "Fetching repositories from $ORG organization..."

# Get all repository names from the organization
REPOS=$(gh repo list "$ORG" --limit 1000 --json name --jq '.[].name')

# Check if any repos were found
if [ -z "$REPOS" ]; then
    echo "No repositories found. Make sure you have access to the $ORG organization."
    exit 1
fi

# Count repos for progress tracking
TOTAL_REPOS=$(echo "$REPOS" | wc -l)
CURRENT=0

echo "Found $TOTAL_REPOS repositories to clone in organization $ORG."
echo "Starting clone process into current directory: $(pwd)"
echo "Starting clone process..."

# Clone each repository
for REPO in $REPOS; do
    CURRENT=$((CURRENT + 1))
    echo "[$CURRENT/$TOTAL_REPOS] Cloning $REPO..."
    
    # Clone the repository directly into the current directory
    gh repo clone "$ORG/$REPO" "$REPO" || {
        echo "Failed to clone $ORG/$REPO"
        continue
    }
    
    echo "Successfully cloned $ORG/$REPO"
done

echo "Completed cloning all accessible repositories from $ORG organization."
echo "Repositories are located in: $(pwd)"
