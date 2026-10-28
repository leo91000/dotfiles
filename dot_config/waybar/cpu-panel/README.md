# CPU and RAM activity panels

Click the CPU or RAM pill on any Waybar to open its panel on that monitor. Click outside,
press Escape, or use the close button to dismiss it. The 1m / 3m / 5m buttons
select the visible history; AVG and PEAK follow that selection. The vertical
scale always represents 0–100% utilization. RAM includes used, available and total
memory in GiB, an allocation meter, and swap usage. Both histories are sampled
by the same service; switching panels keeps their histories.

The panel uses the existing GJS, GTK 3, and gtk-layer-shell packages. It does not
require Eww, AGS, a development Waybar build, or any downloaded dependencies.

- `metrics.js`: reads Linux CPU counters, hardware sensors, and `/proc/meminfo`. Guest ticks are
  not counted twice; idle includes I/O wait. Frequency is the mean of logical
  CPU frequencies. Load is the OS run-queue load, not a percentage. RAM usage is
  `MemTotal - MemAvailable`, so reclaimable cache counts as available. Swap is
  measured against its own capacity; machines without swap show “Not configured”.
- `panel.js`: single-instance GTK application, monitor placement, bounded chart
  history, and interface. Core tiles represent logical CPUs, including SMT.
- `style.css`: panel appearance. The bar pill is styled in `../style.css`.
- `~/.config/systemd/user/waybar-cpu-panel.service`: starts hidden with the
  graphical session, samples once per second, and keeps at most five minutes in
  memory. Detailed sensors and widget redraws run only while the panel is open.

No activity is written to disk. History starts at service startup and gaps after
suspend remain gaps in the chart. Missing temperature/frequency shows a dash.
CPU and RAM are sampled independently; missing RAM data does not produce a false
zero reading. No process names or arguments are read.

```sh
systemctl --user enable --now waybar-cpu-panel.service
systemctl --user restart waybar-cpu-panel.service
journalctl --user -u waybar-cpu-panel.service -n 30
~/.config/waybar/cpu-panel/panel.js          # toggle
~/.config/waybar/cpu-panel/panel.js --memory # RAM panel
~/.config/waybar/cpu-panel/panel.js --hide
gjs -m ~/.config/waybar/cpu-panel/metrics.test.js
```

The original Waybar configuration and stylesheet are backed up in
`~/.local/state/waybar-cpu-backups/20260907/`. After restoring those two files,
restart Waybar using `~/.config/hypr/scripts/start-waybar-triple` and disable the
panel with `systemctl --user disable --now waybar-cpu-panel.service`.

References:
- https://github.com/Alexays/Waybar/wiki/Module:-CPU
- https://wmww.github.io/gtk-layer-shell/
- https://docs.kernel.org/filesystems/proc.html
