---
name: android-emulator-testing
description: Android emulator setup, launch, debugging, and app verification workflows for coding agents. Use when testing Android apps in an emulator, debugging emulator crashes or ADB disconnects, running UIAutomator-based smoke tests, validating Android streaming/reconnect behavior, or launching an emulator from SSH, tmux, Wayland/Hyprland, NVIDIA, or headless-like environments.
---

# Android Emulator Testing

## Workflow

1. Confirm the host graphics state before blaming the app:
   ```bash
   nvidia-smi || true
   cat /proc/driver/nvidia/version 2>/dev/null || true
   printf 'DISPLAY=%s\nWAYLAND_DISPLAY=%s\nXDG_RUNTIME_DIR=%s\n' "$DISPLAY" "$WAYLAND_DISPLAY" "$XDG_RUNTIME_DIR"
   ```
2. Prefer host GPU when a desktop session is available. For Hyprland/Xwayland over SSH, export:
   ```bash
   export XDG_RUNTIME_DIR=/run/user/$(id -u)
   export WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-wayland-1}
   export DISPLAY=${DISPLAY:-:0}
   ```
3. Launch with `scripts/launch-emulator.sh` instead of rewriting emulator boot glue.
4. Drive UI with accessibility/content descriptions via `scripts/tap-node.py`; avoid coordinate taps unless there is no semantic target.
5. Verify a real behavior path. For chat/streaming apps, a valid test must prove the message was sent and a response/tool event completed, not merely that the emulator stayed open.
6. If any instruction in this skill is missing, brittle, or outdated for the current machine/app, ask the user whether to update the skill. When the user agrees, patch this skill before continuing.

## Launch

Use:

```bash
$HOME/.agents/skills/android-emulator-testing/scripts/launch-emulator.sh \
  --avd yama_api_35 \
  --gpu host \
  --apk native-android/app/build/outputs/apk/debug/app-debug.apk
```

The launcher:

- finds `$ANDROID_HOME` or `$HOME/Android/Sdk`
- exports Hyprland/Xwayland display variables when missing
- starts the emulator with `-no-window -no-audio -no-snapshot -no-boot-anim -no-metrics`
- waits for `sys.boot_completed=1`
- optionally installs an APK
- prints the emulator PID, serial, and log path

When launching from an agent shell command, either run inside `tmux` or pass `--hold`; otherwise the harness may clean up background child processes when the command returns:

```bash
$HOME/.agents/skills/android-emulator-testing/scripts/launch-emulator.sh --hold --avd yama_api_35 --gpu host
```

For pure software fallback, use:

```bash
$HOME/.agents/skills/android-emulator-testing/scripts/launch-emulator.sh \
  --avd yama_api_34_nexus4 \
  --gpu off
```

## UI Automation

Use `tap-node.py` after the emulator is booted:

```bash
$HOME/.agents/skills/android-emulator-testing/scripts/tap-node.py --serial emulator-5554 --text YamaCode
$HOME/.agents/skills/android-emulator-testing/scripts/tap-node.py --serial emulator-5554 --desc "New conversation"
$HOME/.agents/skills/android-emulator-testing/scripts/tap-node.py --serial emulator-5554 --desc "Send message"
```

Type text with ADB escaping:

```bash
adb -s emulator-5554 shell input text 'Run%ssleep%s8%susing%sbash%sthen%sreply%sDONE.'
```

Take a screenshot for validation:

```bash
adb -s emulator-5554 shell screencap -p /sdcard/test.png
adb -s emulator-5554 pull /sdcard/test.png /tmp/test.png
```

## Diagnostics

Read `references/troubleshooting.md` when:

- `qemu-system-x86_64-headless` segfaults
- ADB says the device disconnected or waits forever
- `-gpu host` fails from SSH
- NVIDIA driver/library versions are mismatched
- Wayland/Hyprland variables are missing
- software renderers behave differently from host GPU
