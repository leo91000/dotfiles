#!/usr/bin/env sh
set -eu

battery_path=""

for path in /sys/class/power_supply/hidpp_battery_*; do
    [ -r "$path/capacity" ] || continue

    type="$(cat "$path/type" 2>/dev/null || true)"
    [ "$type" = "Battery" ] || continue

    battery_path="$path"
    break
done

if [ -z "$battery_path" ]; then
    jq -cn '{
        text: "",
        tooltip: "No HID++ battery device",
        class: "missing",
        percentage: 0
    }'
    exit 0
fi

capacity="$(cat "$battery_path/capacity" 2>/dev/null || true)"
case "$capacity" in
    "" | *[!0-9]*)
        capacity=0
        ;;
esac

model="$(cat "$battery_path/model_name" 2>/dev/null || basename "$battery_path")"
status="$(cat "$battery_path/status" 2>/dev/null || true)"
name="$(basename "$battery_path")"

if [ "$capacity" -ge 95 ]; then
    icon="󰁹"
    class="good"
elif [ "$capacity" -ge 80 ]; then
    icon="󰂂"
    class="normal"
elif [ "$capacity" -ge 60 ]; then
    icon="󰂀"
    class="normal"
elif [ "$capacity" -ge 40 ]; then
    icon="󰁾"
    class="normal"
elif [ "$capacity" -ge 20 ]; then
    icon="󰁼"
    class="warning"
else
    icon="󰁻"
    class="critical"
fi

text="${capacity}% ${icon}"
tooltip="${model}\n${name}"
[ -n "$status" ] && tooltip="${tooltip}\n${status}"

jq -cn \
    --arg text "$text" \
    --arg tooltip "$tooltip" \
    --arg class "$class" \
    --argjson percentage "$capacity" \
    '{
        text: $text,
        tooltip: $tooltip,
        class: $class,
        percentage: $percentage
    }'
