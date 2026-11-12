//! Headless driver for smoke-testing the supervisor without the UI.

use stackdeck_supervisor::process::{self, ProcSpec};
use stackdeck_supervisor::{discover, provision};

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
        Some("discover") => {
            println!(
                "{}",
                serde_json::to_string_pretty(&discover::scan()).unwrap()
            );
        }
        Some("setup-state") => {
            println!(
                "{}",
                serde_json::to_string_pretty(&provision::detect()).unwrap()
            );
        }
        Some("provision") => {
            let what = args.next().unwrap_or_default();
            let result = match what.as_str() {
                "mariadb" => provision::provision_mariadb(),
                "phpmyadmin" => provision::provision_phpmyadmin(),
                other => {
                    eprintln!("unknown component: {other}");
                    std::process::exit(2);
                }
            };
            if let Err(err) = result {
                eprintln!("{err}");
                std::process::exit(1);
            }
            println!("provisioned {what}");
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
