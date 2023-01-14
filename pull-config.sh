#!/bin/bash
git pull
mkdir -p ~/.config/alacritty
mkdir -p ~/.config/nvim/core
cp ./.config/nvim/init.lua ~/.config/nvim/init.lua
cp -r ./.config/nvim/core ~/.config/nvim/core
cp ./.config/alacritty/alacritty.yml ~/.config/alacritty/alacritty.yml
cp ./.tmux.conf ~/.tmux.conf
cp ./.zshrc ~/.zshrc
