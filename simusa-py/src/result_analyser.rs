use pyo3::ffi::c_str;
use pyo3::prelude::*;
use std::ffi::{CStr, CString};
use std::io::{BufReader, Read, Write};
use std::{fs, net::TcpListener};

use simusa::logger::{LogLevel, LoggerConfig};
use simusa::result_analyser::python_analyser::ResultAnalyserFrame;
use simusa::{
    context::Context,
    errors::{SimusaError, SimusaErrorTypes, SimusaResult},
    info,
    result_analyser::python_analyser::PythonResultAnalyserConfig,
    simulator::Results,
    warning,
};
use simusa::{debug, error};

const CONVERT_TO_DICT: &CStr = cr#"
import json
class NoneDict(dict):
    """ dict subclass that returns a value of None for missing keys instead
        of raising a KeyError. Note: doesn't add item to dictionary.
    """
    def __missing__(self, key):
        return None


def converter(decoded_dict):
    """ Convert any None values in decoded dict into empty NoneDict's. """
    return {k: NoneDict() if v is None else v for k,v in decoded_dict.items()}

def convert(records):
    return json.loads(records, object_hook=converter)
"#;

#[derive(Clone, Debug)]
#[pyclass]
#[pyo3(name = "ResultAnalyser")]
pub struct ResultAnalyser {}

#[pymethods]
impl ResultAnalyser {
    #[new]
    pub fn new() -> Self {
        Self {}
    }

    pub fn run_server(
        &self,
        py: Python,
        host: String,
        port: u16,
        loglevel: String,
    ) -> PyResult<()> {
        let context = Context::new(
            &LoggerConfig {
                log_level: match loglevel.to_lowercase().as_str() {
                    "debug" => LogLevel::Debug,
                    "info" => LogLevel::Info,
                    "warning" | "warn" => LogLevel::Warn,
                    "error" => LogLevel::Error,
                    "off" => LogLevel::Off,
                    _ => LogLevel::Info,
                },
                ..Default::default()
            },
            None,
        );
        let context = context.new_callstack_level("python_result_analyser");
        debug!(
            context,
            "Starting result analyser server at {}:{}", host, port
        );
        let connection = TcpListener::bind(format!("{}:{}", host, port)).map_err(|e| {
            error!(
                context,
                "Impossible to start result analyser server at {}:{}: {}", host, port, e
            );
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Impossible to start result analyser server at {}:{}: {}",
                host, port, e
            ))
        })?;
        if let Some(mut stream) = connection.incoming().flatten().next() {
            let mut buf = Vec::new();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut size: [u8; 8] = Default::default();
            reader.read_exact(&mut size).map_err(|e| {
                let _ = stream.write("OK".as_bytes());
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Impossible to read size request from result analyser client: {}",
                    e
                ))
            })?;
            let size = u64::from_be_bytes(size) as usize;
            buf.resize(size, 0);
            reader.read_exact(&mut buf).map_err(|e| {
                let _ = stream.write("OK".as_bytes());
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Impossible to read request from result analyser client: {}",
                    e
                ))
            })?;
            debug!(context, "Receiving results");
            let frame: ResultAnalyserFrame = serde_json::from_slice(&buf).map_err(|e| {
                let _ = stream.write("OK".as_bytes());
                let mut debug_file = std::fs::File::create("buf_received.json").unwrap();
                debug_file.write_all(&buf).unwrap();
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Impossible to deserialize request from result analyser client: {}",
                    e
                ))
            })?;
            self.analyser_results(py, &frame.results, &frame.config, &context)
                .map_err(|e| {
                    let _ = stream.write("OK".as_bytes());
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Impossible to analyse results: {}",
                        e.detailed_error()
                    ))
                })?;
            let _ = stream.write("OK".as_bytes());
        }
        debug!(context, "Stopping result analyser server");
        Ok(())
    }
}

impl ResultAnalyser {
    fn analyser_results(
        &self,
        py: Python,
        results: &Results,
        config: &PythonResultAnalyserConfig,
        context: &Context,
    ) -> SimusaResult<()> {
        let json_results = serde_json::to_string(&results.records)
            .expect("Error during converting results to json");
        let json_config =
            serde_json::to_string(&results.config).expect("Error during converting config to json");

        let script_path = &config.script_path;
        let python_script = match fs::read_to_string(script_path.clone()) {
            Err(e) => {
                warning!(
                    context,
                    "Result analyser script not found ({}): {}.",
                    script_path,
                    e
                );
                warning!(context, "Creating default result analyser script");
                let mut file = fs::File::create(script_path).map_err(|e| {
                    SimusaError::new(
                        SimusaErrorTypes::ConfigError,
                        format!("Impossible to create default result analyser script: {}", e),
                    )
                })?;
                writeln!(file, "#!/usr/bin/python3")
                    .and_then(|_| writeln!(file, "import numpy as np"))
                    .and_then(|_| writeln!(file, "import matplotlib.pyplot as plt\n"))
                    .and_then(|_| writeln!(file, "# This script is automatically generated by Simusa."))
                    .and_then(|_| writeln!(file, "def {}(results, config, additional_params):", config.function_name))
                    .and_then(|_| writeln!(file,"    print('It works! You can now edit this script to analyse the results.')"))
                    .map_err(|e| {
                        SimusaError::new(
                            SimusaErrorTypes::ConfigError,
                            format!("Impossible to write default result analyser script: {}", e),
                        )
                    })?;
                fs::read_to_string(script_path.clone()).map_err(|e| {
                    SimusaError::new(
                        SimusaErrorTypes::ConfigError,
                        format!("Result analyser script not found ({}): {}", script_path, e),
                    )
                })?
            }
            Ok(s) => s,
        };
        info!(
            context,
            "Calling Python script to analyse the results ({})", script_path
        );
        let python_script = CString::new(python_script).unwrap();
        let res = Self::call_script(
            py,
            &python_script,
            config,
            &json_results,
            &json_config,
            context,
        );
        if let Some(err) = res.err() {
            Err(SimusaError::new(
                SimusaErrorTypes::PythonError,
                err.to_string(),
            ))
        } else {
            Ok(())
        }
    }

    fn call_script(
        py: Python,
        python_script: &CString,
        config: &PythonResultAnalyserConfig,
        json_results: &str,
        json_config: &str,
        context: &Context,
    ) -> PyResult<()> {
        let show_figure_py = cr#"
import matplotlib.pyplot as plt

def show():
    plt.show()
"#;
        let script = PyModule::from_code(py, CONVERT_TO_DICT, c_str!(""), c_str!(""))?;
        let convert_fn: Py<PyAny> = script.getattr("convert")?.into();
        let result_dict = convert_fn.call(py, (json_results,), None)?;
        let config_dict = convert_fn.call(py, (json_config,), None)?;
        let param_dict = convert_fn.call(
            py,
            (serde_json::to_string(&config.additionnal_params).unwrap(),),
            None,
        )?;

        let script = PyModule::from_code(py, python_script, c_str!(""), c_str!(""))?;
        let analyse_fn: Py<PyAny> = script.getattr(&config.function_name)?.into();
        let res = analyse_fn.call(py, (result_dict, config_dict, param_dict), None);
        if let Err(err) = res {
            err.display(py);
            return Err(err);
        }
        if config.show_figures {
            info!(context, "Showing figures...");
            let show_script = PyModule::from_code(py, show_figure_py, c_str!(""), c_str!(""))?;
            let show_fn: Py<PyAny> = show_script.getattr("show")?.into();
            show_fn.call(py, (), None)?;
        }
        Ok(())
    }
}
