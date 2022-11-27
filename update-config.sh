#!/bin/bash

mkdir -p nvim
cp ~/.config/nvim/init.lua ./.config/nvim/init.lua
cp ~/.config/alacritty/alacritty.yml ./.config/alacritty/alacritty.yml
cp ~/.tmux.conf ./.tmux.config
cp ~/.zshrc ./.zshrc
git add .
git commit -m "feat: Update config"
git push
