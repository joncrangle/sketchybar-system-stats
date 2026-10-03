#![cfg(target_os = "macos")]

use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn wait_with_timeout(mut child: Child, timeout: Duration) -> ExitStatus {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("transport fixture exceeded its {timeout:?} timeout");
            }
            Err(error) => panic!("failed to wait for transport fixture: {error}"),
        }
    }
}

#[test]
fn test_sketchybar_transport_regressions() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join("tests/sketchybar_transport_fixture.c");
    let header_fixture = manifest_dir.join("tests/sketchybar_transport_header.c");
    let implementation = manifest_dir.join("include/sketchybar.c");
    let executable = std::env::temp_dir().join(format!(
        "sketchybar-transport-fixture-{}",
        std::process::id()
    ));

    let compile_output = Command::new("cc")
        .arg("-std=c17")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Wpedantic")
        .arg("-Werror")
        .arg("-pthread")
        .arg("-Dbootstrap_look_up=probe_lookup")
        .arg(&fixture)
        .arg(&header_fixture)
        .arg(&implementation)
        .arg("-o")
        .arg(&executable)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to invoke the C compiler for the private Mach fixture");
    assert!(
        compile_output.status.success(),
        "failed to compile private Mach fixture: {}",
        String::from_utf8_lossy(&compile_output.stderr)
    );

    for case in [
        "tokens",
        "delayed-reply",
        "routing",
        "restart",
        "oversized-name",
        "full-queue",
        "null-inputs",
    ] {
        let child = Command::new(&executable)
            .arg(case)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("failed to start private Mach fixture");
        let status = wait_with_timeout(child, Duration::from_secs(6));
        assert!(status.success(), "private Mach fixture failed case {case}");
    }

    let _ = std::fs::remove_file(executable);
}
