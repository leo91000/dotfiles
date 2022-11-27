#!/bin/bash
git pull
mkdir -p ~/.config/alacritty
mkdir -p ~/.config/nvim
cp ./.config/nvim/init.lua ~/.config/nvim/init.lua
cp ./.config/alacritty/alacritty.yml ~/.config/alacritty/alacritty.yml
cp ./.tmux.conf ~/.tmux.conf
cp ./.zshrc ~/.zshrc
