#!/bin/bash

# Ensure yay is installed
if ! command -v yay &> /dev/null; then
    echo "Error: yay is not installed. Please install yay first." >&2
    exit 1
fi

# List of packages
packages=(
    google-chrome
    1password
    lsd
    fd
    ripgrep
    lazygit
    docker-desktop
    jetbrains-toolbox
    neovim
    mise
    chezmoi
    slack-desktop
    ghostty
    steam
    zsh
    tmux
    obsidian
    postgresql-libs
    beeper-latest-bin
    discord
    wl-clipboard
    bottom
    openssl
    jq
    rofi-wayland
    proton-vpn-gtk-app
    qbittorrent
    neofetch
    envycontrol
    cursor-bin
    just
    bat
    git-delta
    htop
)

echo "Installing all packages with yay..."
yay -S --noconfirm --needed --answerdiff None --answerclean None "${packages[@]}"


# Need to find workaround to install zsh autocomplete with deno in TTY
curl -fsSL https://deno.land/install.sh | sh
curl -fsSL https://bun.sh/install | bash
curl -f https://zed.dev/install.sh | sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

#!/bin/bash

# Function to check if AWS CLI is installed
check_aws_cli() {
  if command -v aws &>/dev/null; then
    echo "AWS CLI is already installed."
    aws --version
    return 0
  else
    echo "AWS CLI is not installed."
    return 1
  fi
}

# Function to install AWS CLI
install_aws_cli() {
  echo "Installing AWS CLI..."
  
  # Create a temporary directory for the installation
  local temp_dir=$(mktemp -d)
  cd "$temp_dir"
  
  # Download the AWS CLI installer
  if ! curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip"; then
    echo "Failed to download AWS CLI installer."
    rm -rf "$temp_dir"
    return 1
  fi
  
  # Unzip the installer
  if ! unzip -q awscliv2.zip; then
    echo "Failed to unzip AWS CLI installer. Make sure 'unzip' is installed."
    rm -rf "$temp_dir"
    return 1
  fi
  
  # Install AWS CLI
  if ! sudo ./aws/install; then
    echo "Failed to install AWS CLI."
    rm -rf "$temp_dir"
    return 1
  fi
  
  # Clean up
  cd - > /dev/null
  rm -rf "$temp_dir"
  
  echo "AWS CLI installed successfully."
  aws --version
  return 0
}

# Main script execution
echo "Checking if AWS CLI is installed..."

if ! check_aws_cli; then
  echo "Proceeding with installation..."
  install_aws_cli
fi

echo "Script completed."

