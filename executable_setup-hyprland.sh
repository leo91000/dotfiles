#!/bin/bash
# Make Hyprland the default desktop environment
sudo mkdir -p /etc/gdm/custom.conf.d/
sudo cp /home/leoc/gdm-hyprland.conf /etc/gdm/custom.conf.d/autologin.conf
sudo systemctl restart gdm