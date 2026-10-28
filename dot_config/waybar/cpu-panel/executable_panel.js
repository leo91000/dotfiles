#!/usr/bin/gjs -m
import Gtk from 'gi://Gtk?version=3.0';
import Gdk from 'gi://Gdk?version=3.0';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Layer from 'gi://GtkLayerShell?version=0.1';
import Cairo from 'cairo';
import System from 'system';
import {Metrics} from './metrics.js';

const app = new Gtk.Application({application_id: 'local.waybar.CpuPanel', flags: Gio.ApplicationFlags.HANDLES_COMMAND_LINE});
let window, metrics, graph, hero, summary, temperature, frequency, load, status;
let range = 60;
let mode = 'cpu';
let memoryUsed, memoryAvailable, memoryTotal, memoryBar, memorySplit, swapBar, swapText;
let coreWidgets = [];
let rangeButtons = [];
const views = new Map();

function selectView(requested) {
    // Keep both widget trees alive: accessibility clients can retain references
    // after a panel is hidden. Only the visible view is refreshed.
    views.set(mode, {window, graph, hero, summary, temperature, frequency, load, status,
        range, coreWidgets, rangeButtons, coreGrid, memoryUsed, memoryAvailable,
        memoryTotal, memoryBar, memorySplit, swapBar, swapText});
    window.hide();
    mode = requested;
    const saved = views.get(mode);
    if (saved) {
        ({window, graph, hero, summary, temperature, frequency, load, status,
            range, coreWidgets, rangeButtons, coreGrid, memoryUsed, memoryAvailable,
            memoryTotal, memoryBar, memorySplit, swapBar, swapText} = saved);
        return;
    }
    range = 60;
    rangeButtons = [];
    coreWidgets = [];
    createWindow();
}

function history() {
    return mode === 'memory' ? metrics.memoryHistory : metrics.history;
}

function styled(widget, name) {
    widget.get_style_context().add_class(name);
    return widget;
}
function label(text, name = 'muted') {
    return styled(new Gtk.Label({label: text, xalign: 0}), name);
}
function box(vertical = false, spacing = 0) {
    return new Gtk.Box({orientation: vertical ? Gtk.Orientation.VERTICAL : Gtk.Orientation.HORIZONTAL, spacing});
}
function add(parent, child, expand = false) {
    parent.pack_start(child, expand, expand, 0);
    return child;
}
function button(text, callback, name) {
    const widget = new Gtk.Button({label: text});
    if (name) styled(widget, name);
    widget.connect('clicked', callback);
    return widget;
}
function stat(parent, name, initial) {
    const card = add(parent, styled(box(true, 5), 'stat'), true);
    add(card, label(name, 'muted'));
    return add(card, label(initial, 'stat-value'));
}

function drawChart(area, cr) {
    const width = area.get_allocated_width();
    const height = area.get_allocated_height();
    const left = 0, right = width - 31, top = 12, bottom = height - 24;
    cr.selectFontFace('Noto Sans', Cairo.FontSlant.NORMAL, Cairo.FontWeight.NORMAL);
    cr.setFontSize(9);
    for (const value of [0, 25, 50, 75, 100]) {
        const y = bottom - value / 100 * (bottom - top);
        cr.setSourceRGBA(0.6, 0.72, 0.9, 0.1);
        cr.setLineWidth(1);
        cr.moveTo(left, y + 0.5); cr.lineTo(right, y + 0.5); cr.stroke();
        cr.setSourceRGBA(0.6, 0.69, 0.81, 0.75);
        cr.moveTo(right + 8, y + 3); cr.showText(`${value}`);
    }
    cr.moveTo(0, height - 4); cr.showText(`${range / 60} min ago`);
    cr.moveTo(right - 21, height - 4); cr.showText('now');
    const now = history().at(-1)?.at ?? 0;
    const points = history().filter(item => now - item.at <= range);
    if (points.length < 2) return false;
    // Break the line after suspend or a delayed sample instead of inventing activity.
    const segments = [[]];
    for (const point of points) {
        const segment = segments.at(-1);
        if (segment.length && point.at - segment.at(-1).at > 3) segments.push([]);
        segments.at(-1).push(point);
    }
    const x = point => right - (now - point.at) / range * right;
    const y = point => bottom - point.value / 100 * (bottom - top);
    for (const segment of segments) {
        if (segment.length < 2) continue;
        cr.moveTo(x(segment[0]), bottom);
        for (const point of segment) cr.lineTo(x(point), y(point));
        cr.lineTo(x(segment.at(-1)), bottom); cr.closePath();
        const fill = new Cairo.LinearGradient(0, top, 0, bottom);
        fill.addColorStopRGBA(0, 0.53, 0.79, 1, 0.32);
        fill.addColorStopRGBA(1, 0.53, 0.79, 1, 0.025);
        cr.setSource(fill); cr.fill();
        cr.moveTo(x(segment[0]), y(segment[0]));
        for (const point of segment.slice(1)) cr.lineTo(x(point), y(point));
        cr.setSourceRGBA(0.53, 0.79, 1, 0.08); cr.setLineWidth(7); cr.strokePreserve();
        cr.setSourceRGBA(0.60, 0.84, 1, 1); cr.setLineWidth(1.8); cr.stroke();
    }
    const last = points.at(-1);
    cr.arc(x(last), y(last), 3, 0, Math.PI * 2); cr.setSourceRGB(0.73, 0.90, 1); cr.fill();
    return false;
}

function refresh() {
    const current = history().at(-1);
    if (!current) return;
    const points = history().filter(item => current.at - item.at <= range);
    const average = points.reduce((sum, point) => sum + point.value, 0) / points.length;
    hero.set_text(mode === 'memory' ? Math.floor(current.value).toString() : current.value.toFixed(0));
    summary.set_text(`AVG  ${average.toFixed(0)}%     PEAK  ${Math.max(...points.map(p => p.value)).toFixed(0)}%`);
    if (mode === 'memory') refreshMemory();
    else refreshCpu(current);
    graph.queue_draw();
}

function refreshMemory() {
    const memory = metrics.memory;
    if (!memory) {
        status.set_text('Memory data unavailable');
        return;
    }
    memoryUsed.set_text(`${memory.used.toFixed(1)} GiB`);
    memoryAvailable.set_text(`${memory.available.toFixed(1)} GiB`);
    memoryTotal.set_text(`${memory.total.toFixed(1)} GiB`);
    memoryBar.set_fraction(memory.percentage / 100);
    memorySplit.set_text(`${memory.used.toFixed(1)} GiB used · ${memory.available.toFixed(1)} GiB available`);
    swapBar.set_fraction(memory.swapPercentage / 100);
    swapText.set_text(memory.swapTotal > 0
        ? `${memory.swapUsed.toFixed(1)} / ${memory.swapTotal.toFixed(1)} GiB`
        : 'Not configured');
    status.set_text(memory.percentage >= 95 ? 'Almost full' : memory.percentage >= 80 ? 'Filling up' : 'Plenty of headroom');
}

function refreshCpu(current) {
    const detail = metrics.details();
    temperature.set_text(detail.temperature === null ? '—' : `${detail.temperature.toFixed(0)}°C`);
    frequency.set_text(detail.frequency === null ? '—' : `${detail.frequency.toFixed(2)} GHz`);
    load.set_text(detail.load[0] || '—');
    load.set_tooltip_text(`Load average · 1 / 5 / 15 min\n${detail.load.join(' / ')}`);
    status.set_text(current.value >= 85 ? 'Heavy workload' : current.value >= 40 ? 'Working steadily' : 'Plenty of headroom');
    if (coreWidgets.length !== metrics.cores.length) buildCores();
    for (const [i, value] of metrics.cores.entries()) {
        const {tile, text, bar} = coreWidgets[i];
        text.set_text(`${i.toString().padStart(2, '0')}  ${value.toFixed(0)}%`);
        bar.set_fraction(value / 100);
        tile.set_tooltip_text(`Logical CPU ${i} · ${value.toFixed(1)}%`);
        const css = tile.get_style_context();
        css.remove_class('busy'); css.remove_class('hot');
        if (value >= 85) css.add_class('hot');
        else if (value >= 50) css.add_class('busy');
    }
}

let coreGrid;
function buildCores() {
    for (const child of coreGrid.get_children()) child.destroy();
    coreWidgets = metrics.cores.map((value, i) => {
        const tile = styled(box(true, 5), 'core');
        const text = add(tile, label('00   0%'));
        const bar = add(tile, new Gtk.ProgressBar());
        coreGrid.attach(tile, i % 8, Math.floor(i / 8), 1, 1);
        return {tile, text, bar};
    });
    coreGrid.show_all();
}

function createWindow() {
    const isMemory = mode === 'memory';
    window = new Gtk.ApplicationWindow({application: app, title: isMemory ? 'RAM activity' : 'CPU activity', decorated: false, app_paintable: true});
    window.set_visual(window.get_screen().get_rgba_visual());
    Layer.init_for_window(window);
    Layer.set_namespace(window, 'waybar-cpu-panel');
    Layer.set_layer(window, Layer.Layer.OVERLAY);
    Layer.set_keyboard_mode(window, Layer.KeyboardMode.EXCLUSIVE);
    Layer.set_exclusive_zone(window, -1);
    for (const edge of [Layer.Edge.TOP, Layer.Edge.BOTTOM, Layer.Edge.LEFT, Layer.Edge.RIGHT]) Layer.set_anchor(window, edge, true);
    window.connect('key-press-event', (_, event) => {
        if (event.get_keyval()[1] !== Gdk.KEY_Escape) return false;
        window.hide(); return true;
    });
    window.connect('delete-event', () => { window.hide(); return true; });
    const backdrop = new Gtk.EventBox({visible_window: false});
    backdrop.add_events(Gdk.EventMask.BUTTON_PRESS_MASK);
    backdrop.connect('button-press-event', () => { window.hide(); return true; });
    window.add(backdrop);
    const anchor = box(true);
    backdrop.add(anchor);
    const cardEvents = new Gtk.EventBox({visible_window: false, halign: Gtk.Align.END, valign: Gtk.Align.START, margin_top: 54, margin_end: 14, margin_bottom: 40});
    cardEvents.add_events(Gdk.EventMask.BUTTON_PRESS_MASK);
    cardEvents.connect('button-press-event', () => true);
    add(anchor, cardEvents);
    const panel = styled(box(true, 18), 'panel');
    panel.set_size_request(480, -1);
    cardEvents.add(panel);
    const heading = add(panel, box(false, 12));
    const titles = add(heading, box(true, 5), true);
    add(titles, label(isMemory ? 'SYSTEM / MEMORY' : 'SYSTEM / PROCESSOR', 'eyebrow'));
    add(titles, label(isMemory ? 'RAM activity' : 'CPU activity', 'title'));
    add(heading, label('● LIVE', 'live'));
    const close = add(heading, button('×', () => window.hide(), 'close'));
    close.set_valign(Gtk.Align.CENTER);
    add(panel, label(isMemory ? 'Physical memory · live usage' : metrics.model));
    const heroRow = add(panel, box(false, 7));
    hero = add(heroRow, label('—', 'hero'));
    const unit = add(heroRow, label('%', 'percent')); unit.set_valign(Gtk.Align.END);
    const aside = add(heroRow, box(true, 7), true); aside.set_halign(Gtk.Align.END); aside.set_valign(Gtk.Align.CENTER);
    status = add(aside, label('Sampling activity', 'section'));
    summary = add(aside, label('Waiting for first sample', 'summary'));
    const chartBox = add(panel, box(true, 8));
    const chartHeader = add(chartBox, box(false, 8));
    add(chartHeader, label('UTILIZATION', 'eyebrow'), true);
    const ranges = add(chartHeader, styled(box(false, 2), 'ranges'));
    for (const seconds of [60, 180, 300]) {
        const control = button(`${seconds / 60}m`, () => {
            range = seconds;
            for (const [duration, widget] of rangeButtons) {
                widget.get_style_context()[duration === range ? 'add_class' : 'remove_class']('selected');
            }
            refresh();
        });
        if (seconds === range) styled(control, 'selected');
        rangeButtons.push([seconds, control]); add(ranges, control);
    }
    graph = add(chartBox, new Gtk.DrawingArea({height_request: 166}));
    graph.connect('draw', drawChart);
    const stats = add(panel, box(false, 8)); stats.set_homogeneous(true);
    if (isMemory) buildMemoryDetails(panel, stats);
    else buildCpuDetails(panel, stats);
    add(panel, styled(new Gtk.Separator(), 'divider'));
    const footer = add(panel, box(false));
    add(footer, label('1 sec refresh · 5 min rolling history'), true);
    add(footer, label('esc to close'));
}

function buildCpuDetails(panel, stats) {
    temperature = stat(stats, 'PACKAGE TEMP', '—');
    frequency = stat(stats, 'AVG FREQUENCY', '—');
    load = stat(stats, 'LOAD / 1 MIN', '—');
    const threads = add(panel, box(true, 11));
    const threadHeader = add(threads, box(false));
    add(threadHeader, label('LOGICAL CORES', 'eyebrow'), true);
    add(threadHeader, label(`${metrics.previous.length - 1} threads`));
    coreGrid = add(threads, new Gtk.Grid({column_spacing: 5, row_spacing: 5, column_homogeneous: true}));
    buildCores();
}

function buildMemoryDetails(panel, stats) {
    memoryUsed = stat(stats, 'USED', '—');
    memoryAvailable = stat(stats, 'AVAILABLE', '—');
    memoryTotal = stat(stats, 'TOTAL RAM', '—');
    const allocation = add(panel, box(true, 10));
    add(allocation, label('MEMORY ALLOCATION', 'eyebrow'));
    memoryBar = add(allocation, styled(new Gtk.ProgressBar(), 'memory-meter'));
    memorySplit = add(allocation, label('Waiting for first sample', 'summary'));
    add(allocation, label('Available memory includes reclaimable cache.'));
    const swap = add(panel, styled(box(true, 10), 'stat'));
    const swapHeading = add(swap, box(false));
    add(swapHeading, label('SWAP', 'eyebrow'), true);
    swapText = add(swapHeading, label('—', 'section'));
    swapBar = add(swap, styled(new Gtk.ProgressBar(), 'swap-meter'));
}

function showOnCurrentMonitor() {
    // Query once per opening. Sampling itself never launches subprocesses.
    try {
        const [, out] = GLib.spawn_command_line_sync('hyprctl -j monitors');
        const focused = JSON.parse(new TextDecoder().decode(out)).find(monitor => monitor.focused);
        const display = Gdk.Display.get_default();
        for (let i = 0; i < display.get_n_monitors(); i++) {
            const monitor = display.get_monitor(i);
            const geometry = monitor.get_geometry();
            if (geometry.x === focused?.x && geometry.y === focused?.y) Layer.set_monitor(window, monitor);
        }
    } catch (error) { console.error(`System panel monitor: ${error.message}`); }
    refresh();
    window.show_all();
}

app.connect('startup', () => {
    const css = new Gtk.CssProvider();
    css.load_from_path(GLib.build_filenamev([GLib.get_home_dir(), '.config', 'waybar', 'cpu-panel', 'style.css']));
    Gtk.StyleContext.add_provider_for_screen(Gdk.Screen.get_default(), css, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION);
    metrics = new Metrics();
    app.hold();
    createWindow();
    GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 1, () => {
        if (metrics.sample() && window.get_visible()) refresh();
        return GLib.SOURCE_CONTINUE;
    });
});
app.connect('command-line', (_, command) => {
    try {
        const args = command.get_arguments();
        if (args.includes('--quit')) { app.quit(); return 0; }
        if (args.includes('--background')) return 0;
        if (args.includes('--hide')) { window.hide(); return 0; }
        const requested = args.includes('--memory') ? 'memory' : 'cpu';
        if (requested !== mode) {
            selectView(requested);
            showOnCurrentMonitor();
            return 0;
        }
        if (window.get_visible()) window.hide();
        else showOnCurrentMonitor();
        return 0;
    } finally {
        // GJS is garbage collected: explicitly release each remote launcher.
        command.set_exit_status(0);
        command.done();
    }
});
app.run([System.programInvocationName, ...ARGV]);
