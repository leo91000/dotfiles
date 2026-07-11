#!/usr/bin/env sh
set -eu

swaync-client -swb | while IFS= read -r line; do
    printf '%s\n' "$line" | jq -c '
        ((.text | tonumber?) // 0) as $count
        | {
            text: (
                if $count > 0 then
                    "󰂚 " + ($count | tostring)
                else
                    "󰂜"
                end
            ),
            alt: (.alt // "none"),
            tooltip: (.tooltip // ""),
            class: (.class // "none")
        }
    '
done
