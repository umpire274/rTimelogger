//! rTimeLogger main entrypoint.

use rtimelogger::run;

fn main() {
    println!();
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
