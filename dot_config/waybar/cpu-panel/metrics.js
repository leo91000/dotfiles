import GLib from 'gi://GLib';
import Gio from 'gi://Gio';

export function read(path) {
    try {
        const [ok, data] = GLib.file_get_contents(path);
        return ok ? new TextDecoder().decode(data).trim() : '';
    } catch {
        return '';
    }
}

export function counters(text) {
    return text.split('\n').filter(line => /^cpu\d*\s/.test(line)).map(line => {
        const [id, ...fields] = line.trim().split(/\s+/);
        // guest and guest_nice are already counted in user and nice.
        const values = fields.slice(0, 8).map(Number);
        return {id, total: values.reduce((a, b) => a + b, 0), idle: values[3] + values[4]};
    });
}

export function utilization(before, after) {
    if (!before || before.id !== after.id || after.total <= before.total) return 0;
    return Math.max(0, Math.min(100,
        100 * (1 - (after.idle - before.idle) / (after.total - before.total))));
}

export function memoryUsage(text) {
    const fields = Object.fromEntries([...text.matchAll(/^(\w+):\s+(\d+)\s+kB$/gm)]
        .map(([, key, value]) => [key, Number(value)]));
    if (!(fields.MemTotal > 0) || !Number.isFinite(fields.MemAvailable)) return null;
    const total = fields.MemTotal / 1048576;
    // Available includes reclaimable cache; match Waybar's Linux accounting.
    const available = Math.max(0, Math.min(total, fields.MemAvailable / 1048576));
    const used = total - available;
    const swapTotal = (fields.SwapTotal ?? 0) / 1048576;
    const swapUsed = Math.max(0, Math.min(swapTotal,
        swapTotal - (fields.SwapFree ?? 0) / 1048576));
    return {total, used, available, percentage: used / total * 100,
        swapTotal, swapUsed, swapPercentage: swapTotal > 0 ? swapUsed / swapTotal * 100 : 0};
}

export function appendHistory(history, at, value) {
    if (history.at(-1)?.at > at) history = [];
    return [...history.filter(point => at - point.at <= 300), {at, value}].slice(-301);
}

function temperaturePath() {
    const dir = Gio.File.new_for_path('/sys/class/hwmon').enumerate_children('standard::name', 0, null);
    let entry;
    while ((entry = dir.next_file(null))) {
        const base = `/sys/class/hwmon/${entry.get_name()}`;
        if (!['coretemp', 'k10temp', 'zenpower'].includes(read(`${base}/name`))) continue;
        dir.close(null);
        return `${base}/temp1_input`;
    }
    dir.close(null);
    return null;
}

export class Metrics {
    constructor() {
        this.previous = counters(read('/proc/stat'));
        this.history = [];
        this.memoryHistory = [];
        this.memory = memoryUsage(read('/proc/meminfo'));
        this.cores = [];
        this.sensor = temperaturePath();
        this.model = read('/proc/cpuinfo').match(/model name\s*:\s*(.+)/)?.[1]
            .replaceAll('(R)', '').replaceAll('(TM)', '').replace(/\s+/g, ' ') ?? 'Processor';
    }

    sample() {
        const at = GLib.get_real_time() / 1e6;
        this.memory = memoryUsage(read('/proc/meminfo'));
        if (this.memory) this.memoryHistory = appendHistory(this.memoryHistory, at, this.memory.percentage);
        const next = counters(read('/proc/stat'));
        if (!next.length) return this.memory !== null;
        const previous = new Map(this.previous.map(item => [item.id, item]));
        const values = next.map(item => utilization(previous.get(item.id), item));
        this.cores = values.slice(1);
        this.previous = next;
        this.history = appendHistory(this.history, at, values[0]);
        return true;
    }

    details() {
        const frequencies = [...read('/proc/cpuinfo').matchAll(/cpu MHz\s*:\s*([\d.]+)/g)]
            .map(match => Number(match[1]));
        const temperature = this.sensor ? Number(read(this.sensor)) / 1000 : 0;
        return {
            frequency: frequencies.length ? frequencies.reduce((a, b) => a + b, 0) / frequencies.length / 1000 : null,
            temperature: temperature > 0 ? temperature : null,
            load: read('/proc/loadavg').split(' ').slice(0, 3),
        };
    }
}
