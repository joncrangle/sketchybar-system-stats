fn main() {
    cc::Build::new()
        .file("include/sketchybar.c")
        .compile("system_stats");
}
