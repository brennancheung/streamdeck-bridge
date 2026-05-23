# System Monitoring APIs

> Research date: 2026-05-23
> Part of: Stream Deck XL Direct Control > macOS System Automation

---

## Key Findings

- **`systeminformation`** is the most comprehensive Node.js library: 50+ async functions covering CPU, memory, GPU, disk, network, battery, and temperature with first-class macOS support.
- **CPU usage** is best read via `os.cpus()` delta-tick calculation or `si.currentLoad()` -- no sudo required, ~100ms sampling granularity.
- **Memory** can be read via `os.freemem()`/`os.totalmem()` for basics, or `vm_stat` parsing / `si.mem()` for active/inactive/wired breakdown.
- **GPU usage** on Apple Silicon requires `sudo powermetrics` or IOReport private APIs -- no unprivileged Node.js solution exists.
- **Network throughput** is available per-interface via `si.networkStats()` which gives bytes/sec deltas automatically.
- **Disk I/O** is available via `si.disksIO()` or parsing `iostat` output.
- **Battery** is fully readable without sudo via `pmset -g batt` or `ioreg -rn AppleSmartBattery`.
- **Temperature** on Apple Silicon requires `macos-temperature-sensor` npm package (native addon) -- no sudo needed.
- **Process monitoring** works via `ps aux` parsing or `si.processes()`.
- **Recommended polling rates**: 1-2s for CPU/memory/network, 5s for disk/battery/temperature, 15s+ for GPU (requires sudo).

---

## 1. CPU Usage

### 1.1 Node.js Built-in: `os.cpus()` Delta Calculation

The most portable approach. Take two snapshots of CPU tick counters and compute the percentage of non-idle time between them.

```javascript
const os = require('os');

function cpuSnapshot() {
  const cpus = os.cpus();
  let totalIdle = 0, totalTick = 0;
  for (const cpu of cpus) {
    for (const type in cpu.times) {
      totalTick += cpu.times[type];
    }
    totalIdle += cpu.times.idle;
  }
  return { idle: totalIdle / cpus.length, total: totalTick / cpus.length };
}

// Returns CPU usage as a percentage (0-100)
async function getCpuUsage(intervalMs = 1000) {
  const start = cpuSnapshot();
  await new Promise(r => setTimeout(r, intervalMs));
  const end = cpuSnapshot();
  const idleDelta = end.idle - start.idle;
  const totalDelta = end.total - start.total;
  return Math.round(100 - (100 * idleDelta / totalDelta));
}
```

**Pros**: No dependencies, no sudo, works everywhere.
**Cons**: Blocks for `intervalMs` on first call. Returns aggregate -- no per-core breakdown without extra work.

Per-core usage follows the same pattern but maps over `os.cpus()` individually instead of averaging.

### 1.2 systeminformation

```javascript
const si = require('systeminformation');

const load = await si.currentLoad();
// load.currentLoad        -- aggregate CPU load %
// load.currentLoadUser    -- user-space %
// load.currentLoadSystem  -- kernel %
// load.currentLoadIdle    -- idle %
// load.cpus               -- array of per-CPU breakdowns
```

### 1.3 powermetrics (requires sudo)

```bash
sudo powermetrics -i 1000 -n 1 --samplers cpu_power -f plist
```

Returns per-cluster (E-core/P-core) utilization, frequency residency, and power draw in milliwatts. The plist output format is machine-parseable.

---

## 2. Memory Usage

### 2.1 Node.js Built-in

```javascript
const os = require('os');

function getMemoryInfo() {
  const total = os.totalmem();
  const free = os.freemem();
  const used = total - free;
  return {
    totalGB: (total / 1073741824).toFixed(1),
    usedGB: (used / 1073741824).toFixed(1),
    freeGB: (free / 1073741824).toFixed(1),
    usedPercent: Math.round((used / total) * 100),
  };
}
```

**Caveat**: `os.freemem()` on macOS reports free + inactive pages. It does not distinguish between wired, active, inactive, and compressed memory the way Activity Monitor does.

### 2.2 vm_stat for Detailed Breakdown

For Activity Monitor-style breakdown, parse `vm_stat` output. It reports page counts (multiply by 4096 for bytes): "Pages free", "Pages active", "Pages inactive", "Pages wired down", "Pages occupied by compressor". App memory = active + inactive + speculative + compressed.

### 2.3 systeminformation

```javascript
const mem = await si.mem();
// mem.total       -- total RAM in bytes
// mem.free        -- free RAM
// mem.used        -- used RAM
// mem.active      -- actively used RAM
// mem.available   -- available RAM (free + reclaimable)
// mem.swaptotal   -- swap size
// mem.swapused    -- swap used
```

### 2.4 Memory Pressure (macOS-specific)

```bash
# Returns: The system has normal/warn/critical memory pressure
memory_pressure
```

Useful for a simple green/yellow/red indicator on a Stream Deck button.

---

## 3. GPU Usage

GPU monitoring on macOS is the most challenging metric. Apple does not expose GPU utilization through standard unprivileged APIs.

### 3.1 powermetrics (requires sudo)

```bash
sudo powermetrics -i 2000 -n 1 --samplers gpu_power -f plist
```

Returns GPU active residency percentage, frequency states, and power consumption. This is the most reliable source but requires root.

### 3.2 IOReport Private API (advanced)

The `libIOReport.dylib` private framework provides GPU performance state residency data:

```
IOReportCopyChannelsInGroup("GPU Performance States")
IOReportCreateSubscription(...)
IOReportCreateSamples(...)       // sample 1
IOReportCreateSamplesDelta(...)  // diff between samples
```

This is what `powermetrics` uses internally. Tools like `asitop` and `macmon` wrap this in Rust/Python. No Node.js wrapper exists -- you would need a native addon or shell out to a compiled helper binary.

### 3.4 Practical Approach for Stream Deck

```javascript
const { execSync } = require('child_process');

// Requires sudo -- run the main process with elevated privileges
// or use a privileged helper daemon
function getGpuUsage() {
  try {
    const output = execSync(
      'sudo powermetrics -i 1000 -n 1 --samplers gpu_power 2>/dev/null',
      { timeout: 5000 }
    ).toString();

    const match = output.match(/GPU Active Residency:\s+([\d.]+)%/);
    return match ? parseFloat(match[1]) : null;
  } catch {
    return null; // Not available without sudo
  }
}
```

`si.graphics()` returns static GPU info only (vendor, model, VRAM, core count, Metal version) -- no real-time utilization.

---

## 4. Network Throughput

### 4.1 systeminformation (recommended)

```javascript
const si = require('systeminformation');

// Get default network interface
const defaultIface = await si.networkInterfaceDefault();

// Get network stats -- call repeatedly for rate data
// On second+ call, rx_sec and tx_sec contain bytes/sec
const stats = await si.networkStats(defaultIface);
// stats[0].iface      -- interface name (e.g., "en0")
// stats[0].rx_bytes   -- total received bytes
// stats[0].tx_bytes   -- total transmitted bytes
// stats[0].rx_sec     -- receive rate (bytes/sec) -- available from 2nd call
// stats[0].tx_sec     -- transmit rate (bytes/sec)
// stats[0].rx_dropped -- dropped received packets
// stats[0].tx_dropped -- dropped transmitted packets
```

For manual parsing without `systeminformation`, `netstat -ib` provides per-interface byte counters (columns: Ibytes, Obytes). Sample twice and diff for rates.

### 4.2 macOS Network Metric Caveats

- **1 KiB batching**: macOS applies anti-fingerprinting -- traffic metrics for third-party apps increase in multiples of 1024 bytes. Apple-signed binaries (like `netstat`) are exempt.
- **4 GiB truncation**: A kernel bug (rdar://106029568) can truncate byte counters at the 4 GiB mark for some APIs. The `IFMIB_IFDATA` sysctl path avoids this.
- **`getifaddrs()` is deprecated** for traffic stats on macOS -- its `if_data` struct uses 32-bit fields that overflow quickly.

---

## 5. Disk I/O

### 5.1 systeminformation

```javascript
const io = await si.disksIO();
// io.rIO      -- read operations since boot
// io.wIO      -- write operations since boot
// io.tIO      -- total operations
// io.rIO_sec  -- read ops/sec (from 2nd call)
// io.wIO_sec  -- write ops/sec
// io.tIO_sec  -- total ops/sec
// io.rWaitTime -- read wait time (ms)
// io.wWaitTime -- write wait time (ms)

const disks = await si.fsSize();
// disks[0].fs        -- filesystem identifier
// disks[0].type      -- filesystem type (apfs, hfs, etc.)
// disks[0].size      -- total size in bytes
// disks[0].used      -- used space
// disks[0].available -- available space
// disks[0].use       -- usage percentage
// disks[0].mount     -- mount point
```

For manual approaches: `iostat -c 2 -w 1 disk0` provides KB/transfer, transfers/sec, and MB/sec. `df -g /` gives disk space in GB. Both require parsing subprocess output.

---

## 6. Battery Status

### 6.1 pmset (simplest, no sudo)

```javascript
const { execSync } = require('child_process');

function getBatteryStatus() {
  const output = execSync('pmset -g batt').toString();

  const percentMatch = output.match(/(\d+)%/);
  const stateMatch = output.match(/;\s*(charging|discharging|charged|finishing charge)/);
  const timeMatch = output.match(/(\d+:\d+)\s*remaining/);
  const acMatch = output.includes('AC Power');

  return {
    percent: percentMatch ? parseInt(percentMatch[1]) : null,
    state: stateMatch ? stateMatch[1] : 'unknown',
    timeRemaining: timeMatch ? timeMatch[1] : null,
    acPower: acMatch,
  };
}
```

### 6.2 ioreg for Detailed Battery Info (no sudo)

```javascript
function getDetailedBattery() {
  const output = execSync('ioreg -rn AppleSmartBattery').toString();

  const parse = (key) => {
    const match = output.match(new RegExp(`"${key}"\\s*=\\s*(\\d+)`));
    return match ? parseInt(match[1]) : null;
  };

  const parseBool = (key) => {
    const match = output.match(new RegExp(`"${key}"\\s*=\\s*(Yes|No)`));
    return match ? match[1] === 'Yes' : null;
  };

  return {
    currentCapacity: parse('CurrentCapacity'),       // current charge (mAh)
    maxCapacity: parse('MaxCapacity'),               // current max (degrades over time)
    designCapacity: parse('DesignCapacity'),          // original max
    cycleCount: parse('CycleCount'),                 // charge cycles
    temperature: parse('Temperature') / 100,          // Celsius (raw is centi-degrees)
    voltage: parse('Voltage'),                        // millivolts
    amperage: parse('Amperage'),                      // milliamps (negative = discharging)
    isCharging: parseBool('IsCharging'),
    externalConnected: parseBool('ExternalConnected'),
    fullyCharged: parseBool('FullyCharged'),
    healthPercent: Math.round((parse('MaxCapacity') / parse('DesignCapacity')) * 100),
  };
}
```

`si.battery()` from systeminformation wraps the same data: `percent`, `isCharging`, `acConnected`, `cycleCount`, `timeRemaining`, `voltage`, `currentCapacity`, `maxCapacity`, `designedCapacity`.

---

## 7. Temperature Sensors

### 7.1 macos-temperature-sensor (Apple Silicon, recommended)

```bash
pnpm add macos-temperature-sensor
```

```javascript
const macosTemp = require('macos-temperature-sensor');

function getTemperatures() {
  const temp = macosTemp.temperature();
  // temp.cpu             -- max CPU temperature (Celsius)
  // temp.soc             -- SoC average temperature
  // temp.gpu             -- max GPU temperature
  // temp.cpuDieTemps     -- array of per-die CPU temps
  // temp.gpuDieTemps     -- array of per-die GPU temps
  // temp.probeGroupsTemps -- array of probe group data
  return temp;
}
```

No sudo required. Uses IOHIDEventSystem under the hood for Apple Silicon. The library includes a native addon compiled for arm64.

For Intel Macs, `osx-temperature-sensor` (same author, same API) uses SMC keys instead.

### 7.2 systeminformation Integration

`systeminformation` will automatically use the appropriate temperature sensor package if installed:

```javascript
// Install the companion package first:
// pnpm add macos-temperature-sensor   (Apple Silicon)
// pnpm add osx-temperature-sensor     (Intel)

const temp = await si.cpuTemperature();
// temp.main      -- main CPU temperature
// temp.cores     -- array of per-core temperatures
// temp.max       -- maximum temperature
// temp.socket    -- array of socket temperatures
```

Alternatively, `sudo powermetrics -i 2000 -n 1 --samplers smc` returns fan speeds, die temperatures, and thermal pressure (parseable with `-f plist`).

### 7.3 Apple Silicon Temperature Architecture

Apple Silicon reports temperature through a HID sensor hub rather than SMC directly:

- **IOHIDEventSystem** interface filters sensors by name prefix:
  - `eACC*` -- efficiency core temperatures
  - `pACC*` -- performance core temperatures
- Returns actual Celsius values (not just the nominal/fair/serious/critical buckets that some sources describe)
- On macOS 14+ (Sonoma), some sensors shifted to SMC RPC calls using FourCC keys like `Tp01`, `Tg01`

---

## 8. Process Monitoring

### 8.1 ps-based Process Stats

```javascript
const { execSync } = require('child_process');

function getProcessStats(processName) {
  try {
    const output = execSync(
      `ps aux | grep -i "${processName}" | grep -v grep`
    ).toString();

    return output.split('\n').filter(Boolean).map(line => {
      const cols = line.trim().split(/\s+/);
      return {
        user: cols[0],
        pid: parseInt(cols[1]),
        cpuPercent: parseFloat(cols[2]),
        memPercent: parseFloat(cols[3]),
        rss: parseInt(cols[5]) * 1024, // RSS in bytes (ps reports KB)
        command: cols.slice(10).join(' '),
      };
    });
  } catch {
    return [];
  }
}

// Example: monitor Docker Desktop
const dockerStats = getProcessStats('Docker');
```

For top-N by CPU, use `ps -Arcwwwxo pid,pcpu,pmem,rss,comm | head -6` and parse the columns.

### 8.2 systeminformation

```javascript
const procs = await si.processes();
// procs.all       -- total process count
// procs.running   -- running processes
// procs.blocked   -- blocked processes
// procs.sleeping  -- sleeping processes
// procs.list      -- array of process objects:
//   .pid, .name, .cpu, .mem, .command, .path, .state, .started
```

---

## 9. Node.js Library Comparison

| Library | Coverage | Native Addon | sudo | Notes |
|---------|----------|-------------|------|-------|
| **systeminformation** | CPU, Mem, Disk, Net, Battery, Procs, GPU (static) | No | No | Best all-rounder, 50+ functions |
| **macos-temperature-sensor** | Temperature (Apple Silicon) | Yes | No | CPU, GPU, SoC temps |
| **osx-temperature-sensor** | Temperature (Intel) | Yes | No | SMC-based |
| **node-os-utils** | CPU, Mem, Disk | No | No | Lightweight alternative |
| **node-system-stats** | CPU, Mem, Disk, Net, Battery | No | No | ESM + CJS support |
| **os (built-in)** | Basic CPU, Mem | No | No | No dependencies |

**Recommendation**: Use `systeminformation` as the primary library. Add `macos-temperature-sensor` for Apple Silicon temperature data. Shell out to `powermetrics` via a privileged helper for GPU utilization.

---

## 10. Polling Rates and Performance

### Recommended Intervals by Metric

| Metric | Recommended Interval | Minimum Safe | Notes |
|--------|---------------------|-------------|-------|
| CPU usage | 1-2 seconds | 100ms | Delta calculation needs at least 100ms gap |
| Memory | 1-2 seconds | 500ms | Very cheap to read |
| Network throughput | 1 second | 500ms | Rate calculation needs two samples |
| Disk I/O | 2-5 seconds | 1 second | iostat adds subprocess overhead |
| Disk space | 30-60 seconds | 10 seconds | Changes slowly, df can be expensive on NFS |
| Battery | 5-10 seconds | 2 seconds | Changes very slowly |
| Temperature | 3-5 seconds | 1 second | Native addon has minimal overhead |
| GPU utilization | 5-15 seconds | 2 seconds | Requires sudo powermetrics, heavier |
| Process list | 3-5 seconds | 1 second | ps parsing is moderately expensive |

### Performance Budget

For Stream Deck rendering, you need to leave headroom for image generation and HID writes:

```javascript
// Staggered polling -- avoid sampling everything at once
class SystemMonitor {
  constructor() {
    this.data = {};
    this.intervals = [];
  }

  start() {
    // Fast metrics: 1 second
    this.intervals.push(setInterval(() => this.pollFast(), 1000));
    // Medium metrics: 3 seconds
    this.intervals.push(setInterval(() => this.pollMedium(), 3000));
    // Slow metrics: 10 seconds
    this.intervals.push(setInterval(() => this.pollSlow(), 10000));
  }

  async pollFast() {
    const [load, mem, net] = await Promise.all([
      si.currentLoad(),
      si.mem(),
      si.networkStats(),
    ]);
    this.data.cpu = load.currentLoad;
    this.data.cpuCores = load.cpus.map(c => c.load);
    this.data.memUsed = mem.used;
    this.data.memTotal = mem.total;
    this.data.memPercent = Math.round((mem.used / mem.total) * 100);
    this.data.netRx = net[0]?.rx_sec || 0;
    this.data.netTx = net[0]?.tx_sec || 0;
  }

  async pollMedium() {
    const [temp, procs] = await Promise.all([
      si.cpuTemperature(),
      si.processes(),
    ]);
    this.data.cpuTemp = temp.main;
    this.data.topProcesses = procs.list
      .sort((a, b) => b.cpu - a.cpu)
      .slice(0, 5);
  }

  async pollSlow() {
    const [batt, disk] = await Promise.all([
      si.battery(),
      si.fsSize(),
    ]);
    this.data.batteryPercent = batt.percent;
    this.data.batteryCharging = batt.isCharging;
    this.data.diskUsedPercent = disk[0]?.use || 0;
  }

  stop() {
    this.intervals.forEach(clearInterval);
  }
}
```

Most `si.*` calls complete in 2-25ms (using Node built-ins or lightweight shell commands). The exception is `powermetrics` GPU sampling, which is bound to its sampling interval (1000ms+). At 1-second polling for fast metrics, total CPU overhead is well under 1%.

---

## 11. Stream Deck Display Integration Pattern

The `SystemMonitor` class above provides a `this.data` object that stays current. On each poll cycle, render the updated values to Stream Deck buttons using color-coded gauges (green < 70%, yellow 70-90%, red > 90%). Use `Promise.all()` to batch `si.*` calls within each tier, and `setInterval` to drive the rendering loop at 1-second intervals, reading from the monitor's cached data rather than polling the OS on every frame.

---

## Sources

1. [systeminformation - npm](https://www.npmjs.com/package/systeminformation)
2. [systeminformation - GitHub](https://github.com/sebhildebrandt/systeminformation)
3. [systeminformation - Getting Started](https://systeminformation.io/gettingstarted.html)
4. [macos-temperature-sensor - GitHub](https://github.com/sebhildebrandt/macos-temperature-sensor)
5. [osx-temperature-sensor - npm](https://www.npmjs.com/package/osx-temperature-sensor)
6. [node-os-utils - npm](https://www.npmjs.com/package/node-os-utils)
7. [node-system-stats - npm](https://www.npmjs.com/package/node-system-stats)
8. [CPU Load Calculation with os.cpus() - GitHub Gist](https://gist.github.com/bag-man/5570809)
9. [macOS Network Metrics Using sysctl()](https://milen.me/writings/macos-network-metrics-sysctl-net-rt-iflist2/)
10. [How to get macOS power metrics (IOReport, IOKit, SMC)](https://medium.com/@vladkens/how-to-get-macos-power-metrics-with-rust-d42b0ad53967)
11. [powermetrics man page - ss64](https://ss64.com/mac/powermetrics.html)
12. [macOS vm_stat and memory categories - GitHub](https://github.com/nickdowell/vm_info)
13. [pmset power management - ss64](https://ss64.com/mac/pmset.html)
14. [iostat on macOS - ss64](https://ss64.com/mac/iostat.html)
15. [ioreg AppleSmartBattery - osxdaily](https://osxdaily.com/2024/01/03/how-to-check-battery-capacity-cycle-count-from-command-line-on-mac/)
16. [asitop - Apple Silicon top (GPU monitoring)](https://github.com/tlkh/asitop)
17. [macOS Temperature Sensors - btop DeepWiki](https://deepwiki.com/aristocratos/btop/4.1-macos-temperature-sensors)
18. [macOS Memory Management - CodeLucky](https://codelucky.com/macos-memory-management/)
19. [mac-memory - npm](https://www.npmjs.com/package/mac-memory)
20. [iSMC - Apple SMC CLI tool](https://github.com/dkorunic/iSMC)
