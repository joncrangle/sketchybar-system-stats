extern crate sketchybar_rs;

pub fn send_to_sketchybar(
    flag: &str,
    event: &str,
    vars: Option<String>,
    bar: Option<&String>,
    verbose: bool,
) {
    let command = format!("--{} {} {}", flag, event, vars.unwrap_or_default());

    if let Err(e) = sketchybar_rs::message(&command, bar.map(String::as_str)) {
        eprintln!("Failed to send to SketchyBar: {}", e);
    } else if verbose {
        if let Some(bar_name) = bar {
            println!(
                "Successfully sent to SketchyBar (Bar: {}): {}",
                bar_name, command
            );
        } else {
            println!("Successfully sent to SketchyBar: {}", command);
        }
    }
}
