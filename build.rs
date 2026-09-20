fn main() {
    println!("cargo:rerun-if-changed=include/sketchybar.c");
    println!("cargo:rerun-if-changed=include/sketchybar.h");

    cc::Build::new()
        .file("include/sketchybar.c")
        .compile("sketchybar");
}
