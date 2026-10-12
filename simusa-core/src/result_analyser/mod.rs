//! Result analyser module
//!
//! This module contains the result analyser, which is used to analyse the results of the simulation.
//! The result analyser can be a Python script or a plugin, and is configured through the
//! [`ResultAnalyserConfig`] enum.
//!
//! The result analyser is called at the end of the simulation, when `compute_results`
//! is called, and is passed the results of the simulation (simulation config and records) to
//! be analysed.

use std::sync::Arc;

use simusa_macros::config_derives;

use crate::{
    context::Context,
    errors::{SimusaError, SimusaErrorTypes, SimusaResult},
    plugin_api::PluginAPI,
    result_analyser::python_analyser::{PythonResultAnalyser, PythonResultAnalyserConfig},
    simulator::{Results, SimulatorConfig},
};
#[cfg(feature = "gui")]
use crate::{
    gui::{
        UIComponent,
        utils::{edit_plugin_config, show_plugin_config, string_combobox},
    },
    utils::enum_tools::ToVec,
};

pub mod python_analyser;

/// Result analyser choices
#[config_derives]
pub enum ResultAnalyserConfig {
    /// Python result analyser that starts a Python server to call a custom Python script.
    #[check]
    Python(PythonResultAnalyserConfig),
    /// Plugin result analyser that is defined through the PluginAPI.
    Plugin(serde_json::Value),
}

impl Default for ResultAnalyserConfig {
    fn default() -> Self {
        Self::Python(PythonResultAnalyserConfig::default())
    }
}

#[cfg(feature = "gui")]
impl UIComponent for ResultAnalyserConfig {
    fn show_mut(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        buffer_stack: &mut std::collections::BTreeMap<String, String>,
        global_config: &SimulatorConfig,
        current_node_name: Option<&String>,
        unique_id: &str,
    ) {
        let mut current_str = self.to_string();
        ui.horizontal(|ui| {
            ui.label("Result Analyser:");
            string_combobox(
                ui,
                &ResultAnalyserConfig::to_vec(),
                &mut current_str,
                format!("result-analyser-choice-{}", unique_id),
            );
        });
        if current_str != self.to_string() {
            match current_str.as_str() {
                "Python" => {
                    *self = ResultAnalyserConfig::Python(PythonResultAnalyserConfig::default())
                }
                "Plugin" => *self = ResultAnalyserConfig::Plugin(serde_json::Value::default()),
                _ => panic!("Where did you find this value?"),
            };
        }
        match self {
            ResultAnalyserConfig::Python(python_config) => python_config.show_mut(
                ui,
                ctx,
                buffer_stack,
                global_config,
                current_node_name,
                unique_id,
            ),
            ResultAnalyserConfig::Plugin(config) => edit_plugin_config(
                ui,
                "Plugin Result Analyser",
                "plugin-result-analyser",
                unique_id,
                buffer_stack,
                config,
            ),
        }
    }

    fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str) {
        match self {
            ResultAnalyserConfig::Python(python_config) => python_config.show(ui, ctx, unique_id),
            ResultAnalyserConfig::Plugin(config) => {
                show_plugin_config(ui, "Plugin Result Analyser", config)
            }
        }
    }
}

/// Result analyser choices
pub enum ResultAnalyser {
    /// Python result analyser that starts a Python server to call a custom Python script.
    Python(PythonResultAnalyser),
    /// Plugin result analyser that is defined through the PluginAPI.
    Plugin(Box<dyn ResultAnalyserTrait>),
}

impl ResultAnalyser {
    /// Create a new ResultAnalyser from the given configuration.
    ///
    /// Calls the PluginAPI if the configuration is a `Plugin`.
    pub fn from_config(
        config: &ResultAnalyserConfig,
        global_config: &SimulatorConfig,
        plugin_api: &Option<Arc<dyn PluginAPI>>,
        context: &Context,
    ) -> SimusaResult<Self> {
        Ok(match config {
            ResultAnalyserConfig::Python(python_config) => {
                ResultAnalyser::Python(PythonResultAnalyser::from_config(
                    python_config,
                    global_config,
                    plugin_api,
                    context,
                ))
            }
            ResultAnalyserConfig::Plugin(plugin_analyser_config) => {
                if let Some(plugin_api) = plugin_api {
                    ResultAnalyser::Plugin(plugin_api.get_result_analyser(
                        plugin_analyser_config,
                        global_config,
                        context,
                    ))
                } else {
                    return Err(SimusaError::new(
                        SimusaErrorTypes::PluginAPIError,
                        "Plugin API not set!".to_string(),
                    ));
                }
            }
        })
    }

    /// Analyse the results of the simulation.
    ///
    /// # Arguments
    /// * `results` - The results of the simulation (simulation config and records).
    /// * `context` - The context of the simulation (used for logging).
    pub fn analyse_results(&mut self, results: &Results, context: &Context) -> SimusaResult<()> {
        match self {
            ResultAnalyser::Python(analyser) => analyser.analyse_results(results, context),
            ResultAnalyser::Plugin(analyser) => analyser.analyse_results(results, context),
        }
    }
}

/// Trait for result analysers: at the end of the simulation, when `compute_results` is called, the results are forwarded to the result analyser to be analysed.
/// The result analyser can be a Python script or a plugin.
pub trait ResultAnalyserTrait: std::fmt::Debug + std::marker::Send + std::marker::Sync {
    /// Analyse the results of the simulation.
    ///
    /// # Arguments
    /// * `results` - The results of the simulation (simulation config and records).
    /// * `context` - The context of the simulation (used for logging).
    fn analyse_results(&mut self, results: &Results, context: &Context) -> SimusaResult<()>;
}
