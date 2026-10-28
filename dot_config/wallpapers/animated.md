# Animated desktop wallpapers

Three independent 2560 x 1440, 30 fps H.264 loops, without audio:

| Screen | Wallpaper | Source |
| --- | --- | --- |
| Left: ROG PG279Q | Aurora Lake | https://motionbgs.com/aurora-lake |
| Center: AOC AG274QG | Tranquil Japan Lake View | https://motionbgs.com/tranquil-japan-lake-view |
| Right: Dell AW2721D | Blue Mountain Horizons | https://motionbgs.com/blue-mountain-horizons |

Original 4K downloads: `https://motionbgs.com/dl/4k/7365`, `/dl/4k/7359`, `/dl/4k/7361`.
Videos were resized with FFmpeg, Lanczos scaling, libx264 CRF 19, preset fast.
Files: `~/.local/share/wallpapers/animated/nature/`. The downloaded videos and local binaries are not stored in chezmoi.

## Switch modes

```sh
~/.config/hypr/scripts/start-wallpapers-triple static
~/.config/hypr/scripts/start-wallpapers-triple animated
```

The selected mode is saved in `~/.config/wallpapers/mode`. Hyprland runs the launcher at session startup and after its monitor-toggle shortcut. Outputs are resolved by model and serial number, rather than connector names.

The original static launcher and pictures are preserved. The launcher falls back to them if the animated assets or player are unavailable, or a player fails its startup check.

## Runtime

Each video runs in a `wallpaper-triple-*.service` user service with hardware decoding and restart on failure. `wallpaper-triple-pause.service` checks visible full-screen/maximized windows and display power every two seconds, pausing only the affected screen. Background-workspace games do not pause the visible desktop.

```sh
systemctl --user list-units 'wallpaper-triple-*'
journalctl --user -u wallpaper-triple-pause.service
~/.config/hypr/scripts/pause-wallpapers.rs --once
```

The pause helper is a Cargo nightly script using serde_json 1.0.151.

mpvpaper: https://github.com/GhostNaN/mpvpaper

The player is installed at `~/.local/opt/mpvpaper`, with a wrapper at `~/.local/bin/mpvpaper`. It uses a local extraction of the official Arch mpv 0.41.0-6, mujs 1.3.9-1, libsixel 1.10.5-1 and libxpresent 1.0.2-1 packages. The absolute libmujs dependency was made relocatable using patchelf. System packages were not modified. This local build may need rebuilding after a system library ABI upgrade.

Setup sources and original downloads: `~/.cache/animated-wallpaper-setup/`.
Pre-change live and chezmoi Hyprland configurations: `~/.local/state/wallpapers/backups/2026-09-06/`.
