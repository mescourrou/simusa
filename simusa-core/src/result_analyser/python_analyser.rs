//! Python result analyser that calls a Python script to analyse the results of the simulation.
//!
//! The python script is called by a Python server that is started as an external command.
//! The server is started with the following command:
//! ```python
//! import simusa
//! simusa._internal.ResultAnalyser().run_server('localhost', port, loglevel)
//! ```
//! Then, a TCP connection is established to the server and the results are sent to the server as a JSON message.

#[cfg(feature = "gui")]
use std::collections::BTreeMap;
use std::{
    fs,
    io::{BufReader, BufWriter, Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    sync::Arc,
    time::Duration,
};

#[cfg(feature = "gui")]
use egui::CollapsingHeader;
use serde::{Deserialize, Serialize};
use simusa_macros::config_derives;

use crate::{
    VERSION,
    context::Context,
    errors::{SimusaError, SimusaErrorTypes, SimusaResult},
    info, internal,
    logger::{InternalLog, LogLevel},
    networking::NetworkError,
    plugin_api::PluginAPI,
    result_analyser::ResultAnalyserTrait,
    simulator::{Results, SimulatorConfig},
};

#[cfg(feature = "gui")]
use crate::gui::{
    UIComponent,
    utils::{json_config, path_finder},
};

/// Configuration of the Python Result Analyser.
///
/// The python script is called by a Python server that is started as an external command.
#[config_derives]
pub struct PythonResultAnalyserConfig {
    /// Show the matplotlib figures (call `plt.show()`)
    pub show_figures: bool,
    /// Path to the Python analysis script (path from config location).
    ///
    /// If the file does not exist, a default script (named `analyse_results.py`) is created in the same directory as the config file.
    pub script_path: String,
    /// Name of the function to call in the analysis script (default: `analyse_results`).
    /// This function should have the following signature:
    /// ```def analyse_results(records: list, config: dict, additional_params: dict|None)```
    pub function_name: String,
    /// Arbitrary parameters forwarded to the analysis script (additional_params in the script entry point).
    /// This can be used to forward any custom configuration to the analysis script without having to add it to the simulator configuration.
    pub additionnal_params: crate::utils::GeneralValue,
    /// Port used to communicate with the Python analysis server.
    pub port: u16,
    /// Command used to call Python (default: `python3` for Unix and Mac, `python` for Windows).
    pub python_command: String,
}

impl Default for PythonResultAnalyserConfig {
    fn default() -> Self {
        Self {
            show_figures: false,
            script_path: "analyse_results.py".to_string(),
            function_name: "analyse_results".to_string(),
            additionnal_params: crate::utils::GeneralValue::default(),
            port: 54600,
            python_command: if cfg!(target_os = "windows") {
                "python".to_string()
            } else {
                "python3".to_string()
            },
        }
    }
}

#[cfg(feature = "gui")]
impl UIComponent for PythonResultAnalyserConfig {
    fn show_mut(
        &mut self,
        ui: &mut egui::Ui,
        _ctx: &egui::Context,
        buffer_stack: &mut BTreeMap<String, String>,
        global_config: &SimulatorConfig,
        _current_node_name: Option<&String>,
        unique_id: &str,
    ) {
        let python_param_key = format!("python-result-analyser-config-params-{}", unique_id);
        let python_param_error_key =
            format!("python-result-analyser-config-params-error-{}", unique_id);
        CollapsingHeader::new("Python Result Analyser").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Script path:");
                path_finder(ui, &mut self.script_path, global_config.get_base_path());
            });
            ui.horizontal(|ui| {
                ui.label("Function name:");
                ui.text_edit_singleline(&mut self.function_name);
            });
            ui.horizontal(|ui| {
                ui.label("Show figures:");
                ui.checkbox(&mut self.show_figures, "");
            });
            ui.horizontal(|ui| {
                ui.label("Port:");
                ui.add(egui::DragValue::new(&mut self.port).range(1024..=65535));
            });
            ui.horizontal(|ui| {
                ui.label("Python command:");
                ui.text_edit_singleline(&mut self.python_command);
            });

            ui.vertical(|ui| {
                ui.label("Additional params (JSON format, null for nothing):");
                json_config(
                    ui,
                    &python_param_key,
                    &python_param_error_key,
                    buffer_stack,
                    &mut self.additionnal_params,
                );
            });
        });
    }

    fn show(&self, ui: &mut egui::Ui, _ctx: &egui::Context, _unique_id: &str) {
        CollapsingHeader::new("Python Result Analyser").show(ui, |ui| {
            ui.label(format!("Script path: {}", self.script_path));
            ui.label(format!("Function name: {}", self.function_name));
            ui.horizontal(|ui| {
                ui.label("Show figures: ");
                if self.show_figures {
                    ui.label("Yes");
                } else {
                    ui.label("No");
                }
            });
            ui.horizontal(|ui| {
                ui.label("Port: ");
                ui.label(self.port.to_string());
            });
            ui.horizontal(|ui| {
                ui.label("Python command: ");
                ui.label(self.python_command.clone());
            });
            ui.vertical(|ui| {
                ui.label("Additional params: ");
                ui.label(self.additionnal_params.to_string());
            });
        });
    }
}

/// Python result analyser that calls a Python script to analyse the results of the simulation.
///
/// The python script is called by a Python server that is started as an external command.
/// The server is started with the following command:
/// ```python
/// import simusa
/// simusa._internal.ResultAnalyser().run_server('localhost', port, loglevel)
/// ```
///
/// Then, a TCP connection is established to the server and the results are sent to the server as a JSON message.
#[derive(Debug)]
pub struct PythonResultAnalyser {
    config: PythonResultAnalyserConfig,
    command: Option<std::process::Child>,
}

impl PythonResultAnalyser {
    /// Create a new PythonResultAnalyser from the given configuration.
    pub fn from_config(
        config: &PythonResultAnalyserConfig,
        global_config: &SimulatorConfig,
        _plugin_api: &Option<Arc<dyn PluginAPI>>,
        _context: &Context,
    ) -> Self {
        let config = PythonResultAnalyserConfig {
            script_path: fs::canonicalize(global_config.get_base_path())
                .unwrap()
                .join(&config.script_path)
                .to_str()
                .unwrap()
                .to_string(),
            ..config.clone()
        };
        Self {
            config,
            command: None,
        }
    }

    fn send_request(&self, frame: ResultAnalyserFrame, context: &Context) -> SimusaResult<()> {
        let mut connection = None;
        internal!(
            context,
            InternalLog::ExternalNetwork,
            "Try to establish a connection to the result analyser server at port {}",
            self.config.port
        );
        let max_tries: u8 = 10;
        for i in 0..max_tries {
            if let Ok(conn) = TcpStream::connect(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                self.config.port,
            )) {
                connection = Some(conn);
                break;
            }
            std::thread::sleep(Duration::from_millis(500));
            internal!(
                context,
                InternalLog::ExternalNetwork,
                "Retry to establish a connection ({}/{max_tries} tries)",
                i + 1
            );
        }
        let connection = connection.ok_or_else(|| {
            SimusaError::new(
                SimusaErrorTypes::NetworkError(NetworkError::Other),
                "Impossible to connect to result analyser server".into(),
            )
        })?;
        internal!(context, InternalLog::ExternalNetwork, "plop2");
        let mut reader = BufReader::new(connection.try_clone().unwrap());
        let mut writer = BufWriter::new(connection);
        internal!(
            context,
            InternalLog::ExternalNetwork,
            "Sending results to result analyser server at port {}",
            self.config.port
        );

        let buf = serde_json::to_vec(&frame).map_err(|e| {
            SimusaError::new(
                SimusaErrorTypes::NetworkError(NetworkError::Other),
                format!(
                    "Impossible to serialize results for result analyser server: {}",
                    e
                ),
            )
        })?;
        let _ = writer
            .write(&buf.len().to_be_bytes())
            .expect("Impossible to write size to result analyser server");
        writer.write_all(&buf).map_err(|e| {
            SimusaError::new(
                SimusaErrorTypes::NetworkError(NetworkError::Other),
                format!(
                    "Impossible to send results to result analyser server: {}",
                    e
                ),
            )
        })?;
        writer.flush().unwrap();

        internal!(
            context,
            InternalLog::ExternalNetwork,
            "Waiting for result analyser server response..."
        );

        let mut buf = Vec::new();
        // reader.tim set_read_timeout(None).map_err(|e| {
        //     SimusaError::new(
        //         SimusaErrorTypes::NetworkError(NetworkError::Other),
        //         format!("Impossible to set read timeout for result analyser server: {}", e),
        //     )
        // })?;
        reader.read(&mut buf).map_err(|e| {
            SimusaError::new(
                SimusaErrorTypes::NetworkError(NetworkError::Other),
                format!("Impossible to read from result analyser server: {}", e),
            )
        })?;
        Ok(())
    }
}

impl ResultAnalyserTrait for PythonResultAnalyser {
    fn analyse_results(&mut self, results: &Results, context: &Context) -> SimusaResult<()> {
        info!(context, "Starting Python Result Analyser");
        if self.command.is_some() {
            return Err(SimusaError::new(
                SimusaErrorTypes::NetworkError(NetworkError::Other),
                "Result analyser server is already running".into(),
            ));
        }
        #[cfg(target_os = "windows")]
        let python_distrib = "win_amd64";
        #[cfg(target_os = "linux")]
        let python_distrib = "linux_x86_64";
        #[cfg(target_os = "macos")]
        let python_distrib = "manylinux_2_39_x86_64";

        let python_command = &self.config.python_command;
        self.command = Some(std::process::Command::new(python_command)
            .arg("-c")
            .arg(format!("
try:
    import simusa
except ImportError as e:
    answer = input('simusa python module is not installed, do you accept to install it now? (Y/n) ')
    if answer.lower() == 'y' or answer == '':
        import subprocess
        import sys
        subprocess.check_call([sys.executable, '-m', 'pip', 'install', 'https://gitlab.laas.fr/api/v4/projects/2945/packages/generic/simusa/{0}/simusa-{0}-cp310-abi3-{1}.whl'])
        import simusa
    else:
        raise e

simusa._internal.ResultAnalyser().run_server('127.0.0.1', {2}, '{3}')
            ", VERSION, python_distrib, self.config.port, {
                let l = context.get_loglevel();
                // Convert Internal log level to Debug for Python server,
                // as it does not support Internal log level.
                if let LogLevel::Internal(_) = l {
                    LogLevel::Debug
                } else {
                    l
                }
            }))
            .envs(std::env::vars())
            .spawn()
            .map_err(|e| {
                SimusaError::new(
                    SimusaErrorTypes::NetworkError(NetworkError::Other),
                    format!("Impossible to start result analyser server: {}", e),
                )
            })?
        );

        let frame = ResultAnalyserFrame {
            results: results.clone(),
            config: self.config.clone(),
        };

        let res = self.send_request(frame, context);
        if let Err(err) = &res {
            self.command.as_mut().unwrap().kill().map_err(|e| {
                err.clone()
                    .chain(format!("Impossible to stop result analyser server: {}", e))
            })?;
            self.command = None;
            return Err(err.clone());
        }

        if let Some(mut err) = self.command.as_mut().unwrap().stderr.take() {
            let mut buf = String::new();
            err.read_to_string(&mut buf).unwrap();
            eprintln!("{}", buf);
        }
        if let Some(mut output) = self.command.as_mut().unwrap().stdout.take() {
            let mut buf = String::new();
            output.read_to_string(&mut buf).unwrap();
            println!("{}", buf);
        }

        let result = self.command.as_mut().unwrap().wait().map_err(|e| {
            SimusaError::new(
                SimusaErrorTypes::NetworkError(NetworkError::Other),
                format!("Impossible to end result analyser server: {}", e),
            )
        })?;
        self.command = None;
        if !result.success() {
            return Err(SimusaError::new(
                SimusaErrorTypes::PythonError,
                "Error during the call of Python Result Analyser".to_string(),
            ));
        }
        res
    }
}

impl Drop for PythonResultAnalyser {
    fn drop(&mut self) {
        if let Some(mut command) = self.command.take()
            && let Err(e) = command.kill()
        {
            eprintln!("Impossible to stop result analyser server: {}", e);
        }
    }
}

/// Message frame sent to Python Result Analyser server on python-side.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultAnalyserFrame {
    /// Results of the simulation ([`SimulatorConfig`] and [`Record`](crate::simulator::Record)s).
    pub results: Results,
    /// Configuration of the Python Result Analyser.
    pub config: PythonResultAnalyserConfig,
}
