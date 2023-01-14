#!/bin/bash

mkdir -p .config/nvim/core
cp ~/.config/nvim/init.lua ./.config/nvim/init.lua
cp -r ~/.config/nvim/core ./.config/nvim
cp ~/.config/alacritty/alacritty.yml ./.config/alacritty/alacritty.yml
cp ~/.tmux.conf ./.tmux.conf
cp ~/.zshrc ./.zshrc
git add .
git commit -m "feat: Update config"
git push
