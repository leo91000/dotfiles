# Android Emulator Troubleshooting

## Known Good Yama Code Setup

On this machine, after rebooting into a consistent NVIDIA stack, this worked:

```bash
XDG_RUNTIME_DIR=/run/user/1000 \
WAYLAND_DISPLAY=wayland-1 \
DISPLAY=:0 \
/home/leoc/Android/Sdk/emulator/emulator \
  -avd yama_api_35 \
  -no-window \
  -no-audio \
  -no-snapshot \
  -gpu host \
  -no-boot-anim \
  -no-metrics
```

A valid smoke test created a new YamaCode conversation, sent:

```text
Run sleep 8 using bash then reply DONE. Do not edit files.
```

and confirmed the shell artifact completed and Codex replied `DONE`.

## NVIDIA / Graphics Checks

Run these first:

```bash
nvidia-smi --query-gpu=name,driver_version,display_active --format=csv,noheader
cat /proc/driver/nvidia/version
uname -r
```

If `nvidia-smi` says `Driver/library version mismatch`, reboot or reload the driver stack before emulator debugging. A mismatched NVIDIA userspace/kernel module can make Vulkan/OpenGL emulator paths unstable.

## Hyprland / SSH Display Variables

Physical monitors do not automatically give SSH processes a display. Check:

```bash
pgrep -af 'Hyprland|Xwayland'
ls -la /run/user/$(id -u)/wayland-*
XDG_RUNTIME_DIR=/run/user/$(id -u) WAYLAND_DISPLAY=wayland-1 DISPLAY=:0 glxinfo -B
```

For Hyprland on this machine, `DISPLAY=:0` reached Xwayland and `glxinfo -B` reported the RTX 4090. `hyprctl` may additionally need `HYPRLAND_INSTANCE_SIGNATURE`, but the emulator host-GPU path did not.

## Crash Signatures

Repeated failures looked like:

```text
qemu-system-x86_64-headless SIGSEGV
Graphics backend: gfxstream
Selecting Vulkan device: SwiftShader / llvmpipe
```

Inspect with:

```bash
coredumpctl --no-pager list | rg 'qemu-system-x86'
coredumpctl info <pid>
tail -n 120 /tmp/<emulator-log>.log
```

Pre-reboot or no-display software candidates that crashed during real Yama Code streaming included:

- API 35 Pixel 6, `-gpu swiftshader`
- API 34 Pixel 6/medium phone, `-gpu swiftshader`
- API 34 Pixel 6/medium phone, `-gpu off`
- API 34 Nexus 4, `-gpu swiftshader`
- API 35 Nexus 4, `-gpu off`

The post-reboot host-GPU path is the preferred setup.

## Codex / tmux Process Lifetime

When launched from a Codex command tool, background emulator children may be cleaned up when the shell command returns. For persistent emulator testing, either launch the emulator inside `tmux` or run `launch-emulator.sh --hold` so the command stays attached until the emulator exits.

## Validation Rules

Treat a run as valid only when:

- the app is actually opened past login
- the intended screen is reached
- the action is actually submitted, not left in an input field
- the expected UI or backend effect is observed
- the emulator survives a screenshot or a post-action wait

Use UIAutomator node selection for buttons and rows. Screenshots are useful to confirm visual state, but they do not prove a chat message was submitted.
