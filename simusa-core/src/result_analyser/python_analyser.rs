#[cfg(feature = "gui")]
use std::collections::BTreeMap;
use std::{ffi::{CStr, CString}, fs, io::Write, sync::Arc};

#[cfg(feature = "gui")]
use egui::CollapsingHeader;
use pyo3::{ffi::c_str, prelude::*};
use simusa_macros::config_derives;

use crate::{
    context::Context,
    errors::{SimusaError, SimusaErrorTypes, SimusaResult},
    info,
    plugin_api::PluginAPI,
    result_analyser::ResultAnalyserTrait,
    simulator::Results,
    warning,
};

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

#[cfg(feature = "gui")]
use crate::{
    gui::{
        UIComponent,
        utils::{json_config, path_finder},
    },
    simulator::SimulatorConfig,
};

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
    pub additionnal_params: serde_json::Value,
}

impl Default for PythonResultAnalyserConfig {
    fn default() -> Self {
        Self {
            show_figures: false,
            script_path: "analyse_results.py".to_string(),
            function_name: "analyse_results".to_string(),
            additionnal_params: serde_json::Value::default(),
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
                path_finder(ui, &mut self.script_path, &global_config.base_path);
            });
            ui.horizontal(|ui| {
                ui.label("Function name:");
                ui.text_edit_singleline(&mut self.function_name);
            });
            ui.horizontal(|ui| {
                ui.label("Show figures:");
                ui.checkbox(&mut self.show_figures, "");
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
            ui.vertical(|ui| {
                ui.label("Additional params: ");
                ui.label(self.additionnal_params.to_string());
            });
        });
    }
}

#[derive(Debug, Clone)]
pub struct PythonResultAnalyser {
    config: PythonResultAnalyserConfig,
}

impl PythonResultAnalyser {
    pub fn from_config(
        config: &PythonResultAnalyserConfig,
        global_config: &SimulatorConfig,
        _plugin_api: &Option<Arc<dyn PluginAPI>>,
        _context: &Context,
    ) -> Self {
        let config = PythonResultAnalyserConfig {
            script_path: global_config
                .base_path
                .join(&config.script_path)
                .to_str()
                .unwrap()
                .to_string(),
            ..config.clone()
        };
        Self { config }
    }
}

impl ResultAnalyserTrait for PythonResultAnalyser {
    fn analyse_results(&self, results: &Results, context: &Context) -> SimusaResult<()> {
        info!(context, "Starting Python Result Analyser");

        let json_results = serde_json::to_string(&results.records)
            .expect("Error during converting results to json");
        let json_config =
            serde_json::to_string(&results.config).expect("Error during converting config to json");

        let show_figure_py = cr#"
import matplotlib.pyplot as plt

def show():
    plt.show()
"#;

        let script_path = &self.config.script_path;
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
                writeln!(file, "#!/usr/bin/python3");
                writeln!(file, "import numpy as np");
                writeln!(file, "import matplotlib.pyplot as plt");
                writeln!(file, "");
                writeln!(
                    file,
                    "def {}(results, config, additional_params):",
                    self.config.function_name
                );
                writeln!(
                    file,
                    "    print('It works! You can now edit this script to analyse the results.')"
                );
                fs::read_to_string(script_path.clone()).map_err(|e| {
                    SimusaError::new(
                        SimusaErrorTypes::ConfigError,
                        format!("Result analyser script not found ({}): {}", script_path, e),
                    )
                })?
            }
            Ok(s) => s,
        };
        let python_script = CString::new(python_script).unwrap();
        let res = Python::attach(|py| -> PyResult<()> {
            let script = PyModule::from_code(py, CONVERT_TO_DICT, c_str!(""), c_str!(""))?;
            let convert_fn: Py<PyAny> = script.getattr("convert")?.into();
            let result_dict = convert_fn.call(py, (json_results,), None)?;
            let config_dict = convert_fn.call(py, (json_config,), None)?;
            let param_dict = convert_fn.call(
                py,
                (serde_json::to_string(&self.config.additionnal_params).unwrap(),),
                None,
            )?;

            let script = PyModule::from_code(py, &python_script, c_str!(""), c_str!(""))?;
            let analyse_fn: Py<PyAny> = script.getattr(&self.config.function_name)?.into();
            info!(context, "Calling Python script to analyse the results...");
            let res = analyse_fn.call(py, (result_dict, config_dict, param_dict), None);
            if let Err(err) = res {
                err.display(py);
                return Err(err);
            }
            if self.config.show_figures {
                info!(context, "Showing figures...");
                let show_script = PyModule::from_code(py, show_figure_py, c_str!(""), c_str!(""))?;
                let show_fn: Py<PyAny> = show_script.getattr("show")?.into();
                show_fn.call(py, (), None)?;
            }
            Ok(())
        });
        if let Some(err) = res.err() {
            Err(SimusaError::new(
                SimusaErrorTypes::PythonError,
                err.to_string(),
            ))
        } else {
            Ok(())
        }
    }
}
