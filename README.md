# Simple Sketchybar System Stats

![stats_provider](assets/stats_provider.png)

This is a simple event provider for [Sketchybar](https://github.com/FelixKratz/SketchyBar?tab=readme-ov-file) that sends system stats to Sketchybar via the event trigger `system_stats`.

## Installation

### Prebuilt binaries

You can download a prebuilt binary for Apple Silicon (aarch64) and Intel Macs (x86_64) from the [latest release](https://github.com/joncrangle/sketchybar-system-stats/releases).

### Build locally

1. [Install the Rust toolchain](https://rustup.rs/).

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Use `cargo` to build the binary:

```bash
git clone https://github.com/joncrangle/sketchybar-system-stats.git
cd sketchybar-system-stats
cargo build --release
```

## CLI usage

Use the `help` command to get usage information:
```console
$ stats_provider --help
A simple system stats event provider for Sketchybar.

Usage: stats_provider [OPTIONS]

Options:
  -a, --all                                        Get all stats
  -b, --battery <BATTERY>...                       Get battery stats [possible values: percentage, remaining, state, time_to_full]
  -c, --cpu <CPU>...                               Get CPU stats [possible values: count, frequency, temperature, usage]
  -d, --disk <DISK>...                             Get disk stats [possible values: count, free, total, usage, used]
  -m, --memory <MEMORY>...                         Get memory stats [possible values: ram_available, ram_total, ram_usage, ram_used, swp_free, swp_total, swp_usage, swp_used]
  -n, --network <NETWORK>...                       Network rx/tx in KB/s. Specify network interfaces (e.g., -n eth0 en0 lo0). At least one is required.
  -s, --system <SYSTEM>...                         Get system stats [possible values: arch, distro, host_name, kernel_version, name, os_version, long_os_version]
  -u, --uptime <UPTIME>...                         Get uptime stats [possible values: week, day, hour, min, sec]
  -i, --interval <INTERVAL>                        Refresh interval in seconds [default: 5]
      --network-refresh-rate <NETWORK_REFRESH_RATE> Network refresh rate (how often to refresh network interface list, in stat intervals) [default: 5]
      --bar <BAR>                                  Bar name (optional)
      --verbose                                    Enable verbose output
      --no-units                                   Output values without units
  -h, --help                                       Print help
  -V, --version                                    Print version
```

Example: trigger event with cpu, disk and ram usage percentages at a refresh interval of 2 seconds:
```bash
stats_provider --cpu usage --disk usage --memory ram_usage --interval 2
```

Example: network monitoring with optimized refresh rate:
```bash
# Monitor network with interface refresh every 8 stat intervals
stats_provider --network en0 --interval 3 --network-refresh-rate 8
```

### Uptime Usage

The uptime system supports customizable time units. You can specify which units to display:

```bash
# Show all available units (week, day, hour, min, sec)
stats_provider --uptime

# Show specific units only
stats_provider --uptime day min    # Shows "5d 42m"
stats_provider --uptime week hour  # Shows "2w 5h"
```

Available uptime units:
- `week` (w) - weeks
- `day` (d) - days
- `hour` (h) - hours
- `min` (m) - minutes
- `sec` (s) - seconds

Units are automatically sorted from largest to smallest, with intelligent carry-over (e.g., excess hours carry into days).

### Output Format

By default, all numeric values include their units (MHz, °C, %, GB, KB/s). You can output raw numeric values without units using the `--no-units` flag:

```bash
# With units (default)
stats_provider --cpu usage --memory ram_usage
# Output: CPU_USAGE="45%" RAM_USAGE="60%"

# Without units
stats_provider --cpu usage --memory ram_usage --no-units
# Output: CPU_USAGE="45" RAM_USAGE="60"
```

This is useful when you want to process the values programmatically or apply custom formatting in your Sketchybar configuration.

### Network Optimization

The `--network-refresh-rate` parameter controls how frequently the network interface list is refreshed:

```bash
# Default: refresh network interfaces every 5 stat intervals
stats_provider --network en0 --interval 2 --network-refresh-rate 5

# More frequent refresh (every 2 intervals) for dynamic environments
stats_provider --network en0 wlan0 --network-refresh-rate 2

# Less frequent refresh (every 10 intervals) for stable setups to reduce overhead
stats_provider --network en0 --network-refresh-rate 10
```

**Benefits:**
- **Performance**: Reduces system calls by refreshing interface list less frequently
- **Efficiency**: Network interfaces don't change rapidly, so frequent rescanning is unnecessary
- **Customizable**: Adjust based on your network environment stability

**Recommendation:** Use higher values (8-15) for stable network setups, lower values (2-5) for environments where interfaces frequently change.

Add the `--verbose` flag to see more detailed output:

```console
$ stats_provider --cpu usage --disk usage --memory ram_usage --interval 2 --verbose
SketchyBar Stats Provider is running.
Stats Provider CLI: Cli { all: false, cpu: Some(["usage"]), disk: Some(["usage"]), memory: Some(["ram_usage"]), network: None, system: None, interval: 2, bar: None, verbose: true }
Successfully sent to SketchyBar: --add event system_stats
Current message: CPU_USAGE="4%" DISK_USAGE="65%" RAM_USAGE="54%"
Successfully sent to SketchyBar: --trigger system_stats CPU_USAGE="4%" DISK_USAGE="65%" RAM_USAGE="54%"
Current message: CPU_USAGE="6%" DISK_USAGE="65%" RAM_USAGE="54%"
Successfully sent to SketchyBar: --trigger system_stats CPU_USAGE="6%" DISK_USAGE="65%" RAM_USAGE="54%"
Current message: CPU_USAGE="5%" DISK_USAGE="65%" RAM_USAGE="54%"
Successfully sent to SketchyBar: --trigger system_stats CPU_USAGE="5%" DISK_USAGE="65%" RAM_USAGE="54%"
```

## Usage with Sketchybar

Environment variables that can be provided by the `system_stats` event

| Variable                 | Description                               |
| ------------------------ | ----------------------------------------- |
| `ARCH`                   | System architecture                       |
| `BATTERY_PERCENTAGE`     | Battery charge level %                    |
| `BATTERY_REMAINING`      | Time remaining until empty (min)          |
| `BATTERY_STATE`          | Battery charging state                    |
| `BATTERY_TIME_TO_FULL`   | Time until fully charged (min)            |
| `CPU_COUNT`              | Number of CPU cores                       |
| `CPU_FREQUENCY`          | CPU frequency MHz                         |
| `CPU_TEMP`               | CPU temperature °C                        |
| `CPU_USAGE`              | CPU usage %                               |
| `DISK_COUNT`             | Number of disks                           |
| `DISK_FREE`              | Free disk space GB                        |
| `DISK_TOTAL`             | Total disk space GB                       |
| `DISK_USAGE`             | Disk usage %                              |
| `DISK_USED`              | Used disk space GB                        |
| `DISTRO`                 | System distribution                       |
| `HOST_NAME`              | System host name                          |
| `KERNEL_VERSION`         | System kernel version                     |
| `NETWORK_RX_{INTERFACE}` | Received KB/s from specified interface    |
| `NETWORK_TX_{INTERFACE}` | Transmitted KB/s from specified interface |
| `OS_VERSION`             | System OS version                         |
| `LONG_OS_VERSION`        | System long OS version                    |
| `RAM_TOTAL`              | Total memory GB                           |
| `RAM_AVAILABLE`          | Available memory GB                       |
| `RAM_TOTAL`              | Total memory GB                           |
| `RAM_USAGE`              | Memory usage %                            |
| `RAM_USED`               | Used memory GB                            |
| `SWP_FREE`               | Free swap GB                              |
| `SWP_TOTAL`              | Total swap GB                             |
| `SWP_USAGE`              | Swap usage %                              |
| `SWP_USED`               | Used swap GB                              |
| `SYSTEM_NAME`            | System name (i.e. Darwin)                 |
| `UPTIME`                 | System uptime (customizable units)       |

> [!NOTE]
> System stats that are not expected to change between system restarts (e.g. `NAME`, `OS_VERSION`, etc.) are sent when the app binary starts, but are not refreshed.

### `sketchybarrc` file

Run `stats_provider` with desired options by including it in your `sketchybarrc` config:

```bash
killall stats_provider
# Update with path to stats_provider
$CONFIG_DIR/sketchybar-system-stats/target/release/stats_provider --cpu usage --disk usage --memory ram_usage &
```

Example: use `stats_provider` to add an item `disk_usage`, subscribe to the `system_stats` event and update the `disk_usage` item.

```bash
# Ensure that `stats_provider` is running by invoking it earlier in your `sketchybarrc` file
sketchybar --add item disk_usage right \
           --set disk_usage script="sketchybar --set disk_usage label=\$DISK_USAGE" \
           --subscribe disk_usage system_stats
```

### `SbarLua` module

```lua
-- Update with path to stats_provider
sbar.exec('killall stats_provider >/dev/null; $CONFIG_DIR/sketchybar-system-stats/target/release/stats_provider --cpu usage --disk usage --memory ram_usage')

-- Subscribe and use the `DISK_USAGE` var
local disk = sbar.add('item', 'disk', {
	position = 'right',
})
disk:subscribe('system_stats', function(env)
	disk:set { label = env.DISK_USAGE }
end)
```

## Why?

I wanted a single simple, lightweight binary to provide stats to Sketchybar. I also wanted to learn how to code in Rust, and learning by doing is a great way to learn.

## Thanks

* [Sketchybar](https://github.com/FelixKratz/SketchyBar) and [SbarLua](https://github.com/FelixKratz/SbarLua)
* [sketchybar-rs](https://github.com/johnallen3d/sketchybar-rs)
