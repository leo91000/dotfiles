#!/bin/bash
set -euo pipefail

# Function to check if a package is installed via pacman/yay
is_package_installed() {
    pacman -Q "$1" &> /dev/null
}

# Function to check if a command exists
command_exists() {
    command -v "$1" &> /dev/null
}

# Check if yay is installed
if ! command_exists yay; then
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
    blender
    wofi
    qt6ct
    qt6-base
    qt6-wayland
    kvantum
    kvantum-qt6
    waybar
    hyprland 
    mako 
    dunst 
    thunar 
    pavucontrol 
    pipewire 
    wireplumber 
    xdg-desktop-portal-hyprland 
    hyprpaper
    hyprshot
    hyprpicker
    xone-dkms-git
    xone-dongle-firmware
    zen-browser-bin
    openbsd-netcat
    scc-bin
    brightnessctl
    cliphist
    tokei
    lib32-nvidia-utils
    lib32-vulkan-icd-loader
    nix
    opencode-bin
)

# Filter out already installed packages
packages_to_install=()
for package in "${packages[@]}"; do
    if ! is_package_installed "$package"; then
        packages_to_install+=("$package")
    fi
done

# Install only packages that aren't already installed
if [ ${#packages_to_install[@]} -gt 0 ]; then
    echo "Installing ${#packages_to_install[@]} packages with yay..."
    yay -S --noconfirm --needed --answerdiff None --answerclean None "${packages_to_install[@]}"
else
    echo "All packages are already installed. Skipping installation."
fi

# Function to check and install Deno
install_deno() {
    if command_exists deno; then
        echo "Deno is already installed."
    else
        echo "Installing Deno..."
        curl -fsSL https://deno.land/install.sh | sh
    fi
}

# Function to check and install Bun
install_bun() {
    if command_exists bun; then
        echo "Bun is already installed."
    else
        echo "Installing Bun..."
        curl -fsSL https://bun.sh/install | bash
    fi
}

# Function to check and install Zed
install_zed() {
    if command_exists zed; then
        echo "Zed is already installed."
    else
        echo "Installing Zed..."
        curl -f https://zed.dev/install.sh | sh
    fi
}

# Function to check and install Rust
install_rust() {
    if command_exists rustc && command_exists cargo; then
        echo "Rust is already installed."
    else
        echo "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    fi
}

# Function to check if AWS CLI is installed
check_aws_cli() {
    if command_exists aws; then
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
    
    # Check if unzip is installed
    if ! command_exists unzip; then
        echo "Installing unzip..."
        sudo pacman -S --noconfirm unzip
    fi
    
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
        echo "Failed to unzip AWS CLI installer."
        rm -rf "$temp_dir"
        return 1
    fi
    
    # Install AWS CLI (with update if it exists)
    if command_exists aws; then
        echo "Updating existing AWS CLI installation..."
        if ! sudo ./aws/install --update; then
            echo "Failed to update AWS CLI."
            rm -rf "$temp_dir"
            return 1
        fi
    else
        if ! sudo ./aws/install; then
            echo "Failed to install AWS CLI."
            rm -rf "$temp_dir"
            return 1
        fi
    fi
    
    # Clean up
    cd - > /dev/null
    rm -rf "$temp_dir"
    
    echo "AWS CLI installed/updated successfully."
    aws --version
    return 0
}

# Main script execution
echo "Installing/updating tools..."

install_deno
install_bun
install_zed
install_rust

echo "Checking AWS CLI installation..."
check_aws_cli || install_aws_cli

echo "Script completed successfully."
