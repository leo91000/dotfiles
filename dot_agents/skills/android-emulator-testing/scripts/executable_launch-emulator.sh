#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: launch-emulator.sh [options]

Options:
  --avd NAME          AVD name (default: yama_api_35)
  --gpu MODE          Emulator GPU mode: host, off, swiftshader, etc. (default: host)
  --apk PATH          Optional APK to install after boot
  --port PORT         Emulator console/ADB port (default: 5554)
  --sdk PATH          Android SDK path (default: ANDROID_HOME, ANDROID_SDK_ROOT, or ~/Android/Sdk)
  --log PATH          Emulator log path (default: /tmp/android-emulator-<avd>-<timestamp>.log)
  --window            Do not pass -no-window
  --hold              Keep this launcher attached until the emulator exits
  --timeout SECONDS   Boot timeout (default: 180)
  -h, --help          Show this help
EOF
}

avd="yama_api_35"
gpu="host"
apk=""
port="5554"
sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
log=""
no_window=1
hold=0
timeout_seconds=180

while [[ $# -gt 0 ]]; do
  case "$1" in
    --avd) avd="$2"; shift 2 ;;
    --gpu) gpu="$2"; shift 2 ;;
    --apk) apk="$2"; shift 2 ;;
    --port) port="$2"; shift 2 ;;
    --sdk) sdk="$2"; shift 2 ;;
    --log) log="$2"; shift 2 ;;
    --window) no_window=0; shift ;;
    --hold) hold=1; shift ;;
    --timeout) timeout_seconds="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

adb="$sdk/platform-tools/adb"
emulator="$sdk/emulator/emulator"
serial="emulator-$port"

if [[ ! -x "$adb" ]]; then
  echo "ADB not found or not executable: $adb" >&2
  exit 1
fi

if [[ ! -x "$emulator" ]]; then
  echo "Emulator not found or not executable: $emulator" >&2
  exit 1
fi

export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
if [[ -z "${WAYLAND_DISPLAY:-}" ]]; then
  for candidate in "$XDG_RUNTIME_DIR"/wayland-*; do
    if [[ -S "$candidate" ]]; then
      export WAYLAND_DISPLAY="$(basename "$candidate")"
      break
    fi
  done
fi

if [[ -z "${DISPLAY:-}" ]]; then
  if pgrep -af 'Xwayland :0' >/dev/null 2>&1 || [[ -S /tmp/.X11-unix/X0 ]]; then
    export DISPLAY=":0"
  fi
fi

if [[ -z "$log" ]]; then
  log="/tmp/android-emulator-${avd}-$(date +%s).log"
fi

cmd=(
  "$emulator"
  -avd "$avd"
  -port "$port"
  -no-audio
  -no-snapshot
  -gpu "$gpu"
  -no-boot-anim
  -no-metrics
)

if [[ "$no_window" -eq 1 ]]; then
  cmd+=(-no-window)
fi

echo "Starting emulator"
echo "  avd=$avd"
echo "  serial=$serial"
echo "  gpu=$gpu"
echo "  DISPLAY=${DISPLAY:-}"
echo "  WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-}"
echo "  XDG_RUNTIME_DIR=$XDG_RUNTIME_DIR"
echo "  log=$log"

nohup "${cmd[@]}" >"$log" 2>&1 &
pid=$!
echo "  pid=$pid"

deadline=$((SECONDS + timeout_seconds))
while (( SECONDS < deadline )); do
  if ! kill -0 "$pid" 2>/dev/null; then
    echo "Emulator exited before boot" >&2
    wait "$pid" || true
    tail -n 100 "$log" >&2 || true
    exit 1
  fi

  state="$("$adb" -s "$serial" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r' || true)"
  if [[ "$state" == "1" ]]; then
    echo "Boot completed"
    break
  fi
  sleep 2
done

if [[ "${state:-}" != "1" ]]; then
  echo "Timed out waiting for boot" >&2
  tail -n 100 "$log" >&2 || true
  exit 1
fi

if [[ -n "$apk" ]]; then
  echo "Installing APK: $apk"
  "$adb" -s "$serial" install -r "$apk"
fi

echo "Ready"
echo "  serial=$serial"
echo "  pid=$pid"
echo "  log=$log"

if [[ "$hold" -eq 1 ]]; then
  echo "Holding launcher until emulator exits"
  wait "$pid"
fi
