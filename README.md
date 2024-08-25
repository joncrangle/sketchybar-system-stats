# Simple Sketchybar System Stats

![stats_provider](assets/stats_provider.png)

This is a simple event provider for [Sketchybar](https://github.com/FelixKratz/SketchyBar?tab=readme-ov-file) that sends system stats to Sketchybar as the event trigger `system_stats`.

## Build instructions

At this time it is necessary to compile and install the crate locally. The simplest way to do this is to [install the Rust toolchain](https://rustup.rs/).

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then use `cargo` to build the binary:

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
  -a, --all                   Get all stats
  -c, --cpu [<CPU>...]        [possible values: count, temperature, usage]
  -d, --disk [<DISK>...]      [possible values: count, free, total, usage, used]
  -m, --memory [<MEMORY>...]  [possible values: free, total, usage, used]
  -i, --interval <INTERVAL>   Refresh interval in seconds [default: 5]
      --verbose               Enable verbose output
  -h, --help                  Print help
  -V, --version               Print version
```

Example: trigger event with cpu, disk and memory usage percentages at a refresh interval of 2 seconds:
```bash
stats_provider --cpu usage --disk usage --memory usage --interval 2
```

Add the `--verbose` flag to see more detailed output:

```console
$ stats_provider --cpu usage --disk usage --memory usage --interval 2 --verbose
SketchyBar Stats Provider is running.
Stats Provider CLI: Cli { all: false, cpu: Some(["usage"]), disk: Some(["usage"]), memory: Some(["usage"]), interval: 2, verbose: true }
Current message: CPU_USAGE="4%" MEMORY_USAGE="59%" DISK_USAGE="64%"
Successfully sent to SketchyBar: --trigger system_stats CPU_USAGE="4%" MEMORY_USAGE="59%" DISK_USAGE="64%"
Current message: CPU_USAGE="3%" MEMORY_USAGE="59%" DISK_USAGE="64%"
Successfully sent to SketchyBar: --trigger system_stats CPU_USAGE="3%" MEMORY_USAGE="59%" DISK_USAGE="64%"
Current message: CPU_USAGE="2%" MEMORY_USAGE="59%" DISK_USAGE="64%"
Successfully sent to SketchyBar: --trigger system_stats CPU_USAGE="2%" MEMORY_USAGE="59%" DISK_USAGE="64%"
Current message: CPU_USAGE="7%" MEMORY_USAGE="59%" DISK_USAGE="64%"
Successfully sent to SketchyBar: --trigger system_stats CPU_USAGE="7%" MEMORY_USAGE="59%" DISK_USAGE="64%"
```

## Usage with Sketchybar

### Environment variables that can be provided by the `system_stats` event

Run `stats_provider` with the desired options. Subscribe to the `system_stats` event and use the environment variables to update your Sketchybar items.

| Variable       | Description         |
| -------------- | ------------------- |
| `CPU_COUNT`    | Number of CPU cores |
| `CPU_TEMP`     | CPU usage %         |
| `CPU_USAGE`    | CPU usage %         |
| `DISK_COUNT`   | Number of disks     |
| `DISK_FREE`    | Free disk space GB  |
| `DISK_TOTAL`   | Total disk space GB |
| `DISK_USAGE`   | Disk usage %        |
| `DISK_USED`    | Used disk space GB  |
| `MEMORY_FREE`  | Free memory GB      |
| `MEMORY_TOTAL` | Total memory GB     |
| `MEMORY_USAGE` | Memory usage %      |
| `MEMORY_USED`  | Used memory GB      |

### `sketchybarrc` file

[!TODO] Test this example with a `sketchybarrc` config.

Run `stats_provider` with desired options by including it in your `sketchybarrc` config:

```bash
killall stats_provider
# Update with path to stats_provider
$CONFIG_DIR/sketchybar-system-stats/target/release/stats_provider --cpu usage --disk usage --memory usage &
```

Example: use `stats_provider` to add an item `disk_usage`, subscribe to the `system_stats` event and update the `disk_usage` item.

```bash
sketchybar --add item disk_usage right         \
           --set disk                    \
                 label=$DISK_USAGE       \
           --subscribe disk_usage system_stats
```

### SbarLua module

```lua
-- Update with path to stats_provider
sbar.exec('killall stats_provider >/dev/null; $CONFIG_DIR/sketchybar-system-stats/target/release/stats_provider --cpu usage --disk usage --memory usage')

-- Subscribe and use the `DISK_USAGE` var
local disk = sbar.add('item', 'disk', {
	position = 'right',
})
disk:subscribe('system_stats', function(env)
	disk:set { label = env.DISK_USAGE }
end)
```

## Thanks

* [Sketchybar](https://github.com/FelixKratz/SketchyBar) and [SbarLua](https://github.com/FelixKratz/SbarLua)
* [sketchybar-rs](https://github.com/johnallen3d/sketchybar-rs)
