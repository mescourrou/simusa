//! Simulation result configuration and persisted result payloads.
//!
//! This module defines how results are saved (mode and paths) and the
//! structures used to serialize complete simulation outputs.

#[cfg(feature = "gui")]
use egui::{CollapsingHeader, DragValue};
use simusa_macros::config_derives;
#[cfg(feature = "gui")]
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[cfg(feature = "gui")]
use crate::{
    constants::TIME_ROUND,
    gui::{
        UIComponent,
        utils::{json_config, path_finder, string_combobox},
    },
    utils::enum_tools::ToVec,
};

use crate::{
    result_analyser::ResultAnalyserConfig,
    simulator::{Record, SimulatorConfig},
};

#[config_derives(tag_content)]
#[derive(Default)]
/// Strategy used to save simulation results on disk.
pub enum ResultSaveMode {
    /// Save once when the simulation ends.
    #[default]
    AtTheEnd,
    /// Save continuously as records are produced.
    Continuous,
    /// Save every `N` records.
    Batch(usize),
    /// Save every `period` seconds of simulated time.
    Periodic(f32),
}

#[config_derives]
/// Configuration controlling result persistence and post-processing.
pub struct ResultConfig {
    /// Filename to save the results, in JSON format. The directory of this
    /// file is used to save the figures if results are computed.
    ///
    /// Path from config location.
    ///
    /// If `None`, results are not saved on disk but can still be forwarded to the analysis script.
    pub result_path: Option<String>,

    /// Result save mode.
    pub save_mode: ResultSaveMode,

    /// Select an analyser for the results. If `None`, no analysis is performed.
    #[check]
    pub analyser: Option<ResultAnalyserConfig>,
}

impl Default for ResultConfig {
    /// Default scenario configuration: no nodes.
    fn default() -> Self {
        Self {
            result_path: None,
            save_mode: ResultSaveMode::default(),
            analyser: None,
        }
    }
}

#[cfg(feature = "gui")]
impl UIComponent for ResultConfig {
    fn show_mut(
        &mut self,
        ui: &mut egui::Ui,
        _ctx: &egui::Context,
        buffer_stack: &mut BTreeMap<String, String>,
        global_config: &SimulatorConfig,
        _current_node_name: Option<&String>,
        unique_id: &str,
    ) {
        let python_param_key = format!("result-config-python-params-{}", unique_id);
        let python_param_error_key = format!("result-config-python-params-error-{}", unique_id);
        CollapsingHeader::new("Results").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Result Path:");
                if let Some(path) = &mut self.result_path {
                    path_finder(ui, path, &global_config.base_path);
                    if ui.button("X").clicked() {
                        self.result_path = None;
                    }
                } else if ui.button("+").clicked() {
                    self.result_path = Some(String::from(""));
                }
            });

            let mut current_str = self.save_mode.to_string();
            ui.horizontal(|ui| {
                ui.label("Save mode:");
                string_combobox(
                    ui,
                    &ResultSaveMode::to_vec(),
                    &mut current_str,
                    format!("result-save-mode-choice-{}", unique_id),
                );
                if let ResultSaveMode::Batch(size) = &mut self.save_mode {
                    ui.add(DragValue::new(size).max_decimals(0));
                    if *size < 1 {
                        *size = 1;
                    }
                } else if let ResultSaveMode::Periodic(period) = &mut self.save_mode {
                    ui.add(DragValue::new(period));
                    if *period <= TIME_ROUND {
                        *period = TIME_ROUND;
                    }
                }
            });
            if current_str != self.save_mode.to_string() {
                match current_str.as_str() {
                    "AtTheEnd" => self.save_mode = ResultSaveMode::AtTheEnd,
                    "Continuous" => self.save_mode = ResultSaveMode::Continuous,
                    "Batch" => self.save_mode = ResultSaveMode::Batch(10),
                    "Periodic" => self.save_mode = ResultSaveMode::Periodic(5.),
                    _ => panic!("Where did you find this value?"),
                };
            }

            if let Some(analyser) = &mut self.analyser {
                analyser.show_mut(
                    ui,
                    _ctx,
                    buffer_stack,
                    global_config,
                    _current_node_name,
                    unique_id,
                );
                if ui.button("- Remove analyser").clicked() {
                    self.analyser = None;
                }
            } else if ui.button("+ Add analyser").clicked() {
                self.analyser = Some(ResultAnalyserConfig::default());
            }
        });
    }

    fn show(&self, ui: &mut egui::Ui, _ctx: &egui::Context, _unique_id: &str) {
        CollapsingHeader::new("Results").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Result Path: ");
                if let Some(path) = &self.result_path {
                    ui.label(path);
                } else {
                    ui.label("None");
                }
            });

            ui.horizontal(|ui| {
                let mut as_str = self.save_mode.to_string();
                if let ResultSaveMode::Batch(s) = &self.save_mode {
                    as_str = format!("{} ({})", as_str, s);
                } else if let ResultSaveMode::Periodic(p) = &self.save_mode {
                    as_str = format!("{} ({} s)", as_str, p);
                }
                ui.label(format!("Save mode: {}", as_str));
            });

            if let Some(analyser) = &self.analyser {
                analyser.show(ui, _ctx, _unique_id);
            } else {
                ui.label("No analyser configured.");
            }
        });
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Persisted simulation output containing config and produced records.
pub struct Results {
    /// Simulator configuration used to produce these results.
    pub config: SimulatorConfig,
    /// Recorded events and states generated during simulation.
    pub records: Vec<Record>,
}

#[derive(Clone)]
pub(super) struct ResultSavingData {
    pub save_mode: ResultSaveMode,
    pub first_row: bool,
}

impl Default for ResultSavingData {
    fn default() -> Self {
        Self {
            save_mode: ResultSaveMode::default(),
            first_row: true,
        }
    }
}
