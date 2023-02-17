#!/bin/bash
git pull
mkdir -p ~/.config/alacritty
mkdir -p ~/.config/nvim/lua
cp ./.config/nvim/init.lua ~/.config/nvim/init.lua
cp ./.config/nvim/lazy-lock.json ~/.config/nvim/lazy-lock.json
cp -r ./.config/nvim/lua ~/.config/nvim
cp ./.config/alacritty/alacritty.yml ~/.config/alacritty/alacritty.yml
cp ./.tmux.conf ~/.tmux.conf
cp ./.zshrc ~/.zshrc
