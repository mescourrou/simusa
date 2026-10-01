use std::sync::Arc;

use simusa_macros::config_derives;

#[cfg(feature = "gui")]
use crate::{gui::{UIComponent, utils::{json_config, string_combobox}}, utils::enum_tools::ToVec};
use crate::{context::Context, errors::{SimusaError, SimusaErrorTypes, SimusaResult}, plugin_api::PluginAPI, result_analyser::python_analyser::{PythonResultAnalyser, PythonResultAnalyserConfig}, simulator::{Record, Results, SimulatorConfig}};

pub mod python_analyser;


#[config_derives]
pub enum ResultAnalyserConfig {
    #[check]
    Python(PythonResultAnalyserConfig),
    External(serde_json::Value),
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
    )
    {
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
                "Python" => *self = ResultAnalyserConfig::Python(PythonResultAnalyserConfig::default()),
                "External" => {
                    *self = ResultAnalyserConfig::External(
                        serde_json::Value::default(),
                    )
                }
                _ => panic!("Where did you find this value?"),
            };
        }
        match self {
            ResultAnalyserConfig::Python(python_config) => python_config.show_mut(ui, ctx, buffer_stack, global_config, current_node_name, unique_id),
            ResultAnalyserConfig::External(config) => {
                egui::CollapsingHeader::new("External Result Analyser").show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label("Config (JSON):");
                        json_config(
                            ui,
                            &format!("external-result-analyser-key-{}", &unique_id),
                            &format!("external-result-analyser-error-key-{}", &unique_id),
                            buffer_stack,
                            config,
                        );
                    });
                });
            }
        }
    }

    fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str) {
        match self {
            ResultAnalyserConfig::Python(python_config) => python_config.show(ui, ctx, unique_id),
            ResultAnalyserConfig::External(config) => {
                egui::CollapsingHeader::new("External Result Analyser").show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label("Config (JSON):");
                        ui.label(format!("{}", config));
                    });
                });
            }
        }
    }
}

pub enum ResultAnalyser {
    Python(PythonResultAnalyser),
    External(Box<dyn ResultAnalyserTrait>),
}

impl ResultAnalyser {
    pub fn from_config(config: &ResultAnalyserConfig,  global_config: &SimulatorConfig, plugin_api: &Option<Arc<dyn PluginAPI>>, context: &Context) -> SimusaResult<Self> {
        Ok(match config {
            ResultAnalyserConfig::Python(python_config) => {
                ResultAnalyser::Python(PythonResultAnalyser::from_config(python_config, global_config, plugin_api, context))
            }
            ResultAnalyserConfig::External(external_analyser_config) => {
                if let Some(plugin_api) = plugin_api {
                    ResultAnalyser::External(plugin_api.get_result_analyser(
                        external_analyser_config,
                        global_config,
                        context,
                    ))
                } else {
                    return Err(SimusaError::new(
                        SimusaErrorTypes::ExternalAPIError,
                        "Plugin API not set!".to_string(),
                    ));
                }
            }
        })
    }

    pub fn analyse_results(&self, results: &Results, context: &Context) -> SimusaResult<()> {
        match self {
            ResultAnalyser::Python(analyser) => analyser.analyse_results(results, context),
            ResultAnalyser::External(analyser) => analyser.analyse_results(results, context),
        }
    }
}


pub trait ResultAnalyserTrait : std::fmt::Debug + std::marker::Send + std::marker::Sync {
    fn analyse_results(&self, results: &Results, context: &Context) -> SimusaResult<()>;
}