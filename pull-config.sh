#!/bin/bash
git pull
mkdir -p ~/.config/nvim/lua
cp ./nvim/init.lua ~/.config/nvim/init.lua
cp -r ./nvim/lua ~/.config/nvim/lua
