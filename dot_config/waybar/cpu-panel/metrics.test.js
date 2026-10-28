import GLib from 'gi://GLib';
import {counters, utilization, memoryUsage, appendHistory, Metrics} from './metrics.js';

function assert(condition, message) {
    if (!condition) throw new Error(message);
}
const before = counters('cpu 100 20 30 400 10 5 5 0 40 10\ncpu0 1 0 1 8 0 0 0 0\nintr 50');
assert(before.length === 2, 'Only CPU rows should be parsed');
assert(before[0].total === 570, 'Guest time must not be counted twice');
assert(before[0].idle === 410, 'I/O wait is included in idle');
const after = counters('cpu 150 20 40 430 20 5 5 0 40 10')[0];
assert(Math.abs(utilization(before[0], after) - 60) < 0.001, 'Busy delta should be 60%');
assert(utilization(after, after) === 0, 'Unchanged counters must not divide by zero');
assert(utilization(undefined, after) === 0, 'New cores need a baseline');
assert(utilization(after, before[0]) === 0, 'Reset counters need a baseline');
const live = new Metrics();
GLib.usleep(150000);
assert(live.sample(), 'Live /proc/stat sample should succeed');
assert(live.cores.length > 0 && live.cores.every(value => value >= 0 && value <= 100), 'Core usage must be bounded');
live.history.unshift({at: GLib.get_real_time() / 1e6 - 301, value: 50});
live.sample();
assert(live.history.every(point => GLib.get_real_time() / 1e6 - point.at <= 300), 'History must expire after five minutes');
live.history.push({at: GLib.get_real_time() / 1e6 + 1000, value: 50});
live.sample();
assert(live.history.length === 1, 'Reset history when the clock moves backwards');
print(`PASS: counter deltas, guest ticks, resets, ${live.cores.length} live cores, history bounds and clock changes`);

const memory = memoryUsage('MemTotal: 8388608 kB\nMemAvailable: 5242880 kB\nMemFree: 1048576 kB\nSwapTotal: 4194304 kB\nSwapFree: 3145728 kB');
assert(memory.total === 8 && memory.used === 3 && memory.available === 5, 'Use available memory, not just free pages; convert KiB to GiB');
assert(memory.percentage === 37.5, 'RAM percentage should match used / total');
assert(memory.swapUsed === 1 && memory.swapPercentage === 25, 'Swap utilization should use swap capacity');
const noSwap = memoryUsage('MemTotal: 8388608 kB\nMemAvailable: 8388608 kB');
assert(noSwap.used === 0 && noSwap.swapTotal === 0 && noSwap.swapPercentage === 0, 'No swap must not divide by zero');
assert(memoryUsage('MemTotal: 0 kB\nMemAvailable: 0 kB') === null, 'Zero total is invalid');
assert(memoryUsage('MemTotal: 10 kB') === null && memoryUsage('') === null, 'Missing data must not produce fake zero usage');
const full = memoryUsage('MemTotal: 8388608 kB\nMemAvailable: 0 kB');
assert(full.used === 8 && full.percentage === 100, 'Fully used RAM should remain bounded');
assert(appendHistory(Array.from({length: 500}, () => ({at: 1000, value: 1})), 1000, 2).length === 301, 'History must have an absolute size bound');
assert(live.memory && live.memoryHistory.length > 0, 'Live RAM history should sample alongside CPU');
print('PASS: RAM accounting, GiB units, swap, missing readings, full memory and bounded live history');
