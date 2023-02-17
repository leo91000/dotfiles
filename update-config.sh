#!/bin/bash

mkdir -p .config/nvim/lua
cp ~/.config/nvim/init.lua ./.config/nvim/init.lua
cp -r ~/.config/nvim/lua ./.config/nvim
cp ~/.config/alacritty/alacritty.yml ./.config/alacritty/alacritty.yml
cp ~/.tmux.conf ./.tmux.conf
cp ~/.zshrc ./.zshrc
git add .
git commit -m "feat: Update config"
git push
