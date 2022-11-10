#!/bin/bash

mkdir -p nvim
cp ~/.config/nvim/init.lua ./nvim
cp -r ~/.config/nvim/lua ./nvim
git add .
git commit -m "feat: Update config"
git push
