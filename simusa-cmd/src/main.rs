use clap::Parser;
use std::{
    path::Path,
    process::{ExitCode, Termination},
};

use simusa::{errors::SimusaResult, gui, simulator::Simulator};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Disable GUI: will load the config, run the simulation and compute the results
    #[arg(long, default_value_t = false)]
    no_gui: bool,
    /// Default config path. Required if no-gui is used
    config_path: Option<String>,
    /// Load result from the result file specified in the configuration. Without GUI, processes the results directly.
    #[arg(long, default_value_t = false)]
    load_results: bool,
}

fn doit(args: Cli) -> SimusaResult<()> {
    if args.no_gui {
        if args.config_path.is_none() {
            println!("If no gui is used, provide a config path!");
            return Ok(());
        }
        println!("Load configuration...");
        let mut simulator = Simulator::from_config_path(
            Path::new(&args.config_path.unwrap()),
            None, //<- plugin API, to load external modules
        )?;

        // Show the simulator loaded configuration
        println!("Configuration loaded:");
        simulator.show_config();

        if !args.load_results {
            // Run the simulator for the time given in the configuration
            // It also save the results to json
            simulator.run()?;
        } else {
            simulator.load_results(None)?;
        }
        simulator.compute_results()?;

        return Ok(());
    }

    gui::run_gui(args.config_path, None, args.load_results);
    Ok(())
}

fn main() -> ExitCode {
    let args = Cli::parse();

    let res = doit(args);
    if let Err(e) = res {
        println!("{}", e.detailed_error());
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
