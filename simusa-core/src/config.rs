//! Shared low-level configuration primitives used across simulator modules.
//!
//! This module currently defines [`NumberConfig`], which represents either a fixed scalar value
//! or a sampled random-variable configuration.

use simusa_macros::config_derives;

#[cfg(feature = "gui")]
use crate::{constants::TIME_ROUND_DECIMALS, gui::UIComponent};
use crate::utils::determinist_random_variable::RandomVariableTypeConfig;

/// Numeric configuration value.
///
/// This enum is typically used in higher-level `*Config` structures when a field can be either a
/// fixed number or a random variable.
///
/// Default: [`NumberConfig::Num`] with `0.0`.
#[config_derives(tag_content)]
pub enum NumberConfig {
    /// Deterministic numeric value.
    Num(f32),
    /// Random variable description to sample numeric values from.
    #[check]
    Rand(RandomVariableTypeConfig),
}

impl Default for NumberConfig {
    fn default() -> Self {
        Self::Num(0.0)
    }
}

#[cfg(feature = "gui")]
impl UIComponent for NumberConfig {
    fn show_mut(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        buffer_stack: &mut std::collections::BTreeMap<String, String>,
        global_config: &crate::simulator::SimulatorConfig,
        current_node_name: Option<&String>,
        unique_id: &str,
    )
    {
        match self {
            NumberConfig::Num(num) => {
                ui.add(egui::DragValue::new(num).max_decimals(TIME_ROUND_DECIMALS));
            }
            NumberConfig::Rand(rand_config) => {
                rand_config.show_mut(
                    ui,
                    ctx,
                    buffer_stack,
                    global_config,
                    current_node_name,
                    unique_id,
                );
            }
        }
        if ui.button("Switch number type").clicked() {
            *self = match self {
                NumberConfig::Num(_) => {
                    NumberConfig::Rand(RandomVariableTypeConfig::default())
                }
                NumberConfig::Rand(_) => NumberConfig::Num(1.),
            }
        }
    }

    fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str) {
        match self {
            NumberConfig::Num(num) => {
                ui.label(format!("{}", num));
            }
            NumberConfig::Rand(rand_config) => {
                rand_config.show(ui, ctx, unique_id);
            }
        }
    }
}