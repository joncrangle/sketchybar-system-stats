# Simple Sketchybar System Stats

![stats_provider](assets/stats_provider.png)

This is a simple event provider for [Sketchybar](https://github.com/FelixKratz/SketchyBar?tab=readme-ov-file) that sends the current CPU, memory and disk usage percentages to Sketchybar every 5 seconds as the event `system_stats`.

## Build instructions

At this time it is necessary to compile and install the crate locally. The simplest way to do this is to [install the Rust toolchain](https://rustup.rs/).

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then use cargo to build the binary:

```bash
cargo build --release
```

## Sample usage with SbarLua

The following example shows how to use the `stats_provider` event provider to get the disk usage percentage.
```lua
-- Update with path to stats_provider
sbar.exec('killall stats_provider >/dev/null; $CONFIG_DIR/stats_provider/target/release/stats_provider')

-- Subscribe and use the `DISK_USAGE` var
local disk = sbar.add('item', 'disk', {
	position = 'right',
	icon = {
		font = 'A nerd font'
		string = '',
		padding_right = 0
	},
})
disk:subscribe('system_stats', function(env)
	disk:set { label = env.DISK_USAGE }
end)
```

CPU and memory usage can be obtained in the same way by using the `CPU_USAGE` and `MEMORY_USAGE` vars.

## Thanks

* [Sketchybar](https://github.com/FelixKratz/SketchyBar) and [SbarLua](https://github.com/FelixKratz/SbarLua)
* [sketchybar-rs](https://github.com/johnallen3d/sketchybar-rs)
