//! Headless driver for smoke-testing the supervisor without the UI.

use stackdeck_supervisor::process::{self, ProcSpec};

fn main() {
    let mut args = std::env::args().skip(1);
    let log_dir = std::env::temp_dir().join("stackdeck-logs");

    match args.next().as_deref() {
        Some("start") => {
            let Some(name) = args.next() else { usage() };
            let Some(program) = args.next() else { usage() };
            let mut spec = ProcSpec::new(&name, program);
            spec.args = args.collect();
            match process::spawn(&spec, &log_dir) {
                Ok(spawned) => println!("{}", spawned.pid),
                Err(err) => {
                    eprintln!("{err}");
                    std::process::exit(1);
                }
            }
        }
        Some("stop") => {
            let pid = parse_pid(args.next());
            match process::stop(pid) {
                Ok(()) => println!("stopped {pid}"),
                Err(err) => {
                    eprintln!("{err}");
                    std::process::exit(1);
                }
            }
        }
        Some("alive") => {
            println!("{}", process::alive(parse_pid(args.next())));
        }
        _ => usage(),
    }
}

fn parse_pid(arg: Option<String>) -> u32 {
    match arg.and_then(|v| v.parse().ok()) {
        Some(pid) => pid,
        None => usage(),
    }
}

fn usage() -> ! {
    eprintln!("usage: sd start <name> <program> [args...] | sd stop <pid> | sd alive <pid>");
    std::process::exit(2)
}
