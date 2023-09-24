mod cli;
mod ui;
use rs_timeskip_archiver::establish_connection;
use std::env;



fn main() {
    let mut connection = establish_connection();
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && args[1] == "cli" {
        cli::run_cli(&mut connection);
    } else {
        ui::run_ui(connection);
    }
}