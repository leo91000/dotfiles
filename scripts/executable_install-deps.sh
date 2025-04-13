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
)

echo "Installing all packages with yay..."
yay -S --noconfirm --needed --nodiffmenu "${packages[@]}"


# Need to find workaround to install zsh autocomplete with deno in TTY
curl -fsSL https://deno.land/install.sh | sh
curl -fsSL https://bun.sh/install | bash
curl -f https://zed.dev/install.sh | sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

echo "All done!"
