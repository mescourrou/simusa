//! Physics module.
//!
//! This module defines the [`Physics`] trait, physics configuration/record enums,
//! service request/response types, and factory helpers used to instantiate runtime physics
//! implementations.
//! Implementations may include perfect/internal physics, external plugin-backed physics,
//! or Python-backed physics.

pub mod internal_physics;
pub mod robot_models;

pub mod fault_models;

use std::sync::{Arc, RwLock};

use config_checker::*;
use serde_derive::{Deserialize, Serialize};
use simusa_macros::config_derives;

/// Enumeration of the different physic implementations.
#[config_derives]
pub enum PhysicsConfig {
    /// Built-in Rust internal physics implementation.
    #[check]
    Internal(internal_physics::InternalPhysicConfig),
    /// Plugin-provided physics implementation.
    Plugin(crate::utils::GeneralValue),
}

#[cfg(feature = "gui")]
impl UIComponent for PhysicsConfig {
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
            ui.label("Physics:");
            string_combobox(
                ui,
                &PhysicsConfig::to_vec(),
                &mut current_str,
                format!("physics-choice-{}", unique_id),
            );
        });
        if current_str != self.to_string() {
            match current_str.as_str() {
                "Internal" => {
                    *self =
                        PhysicsConfig::Internal(internal_physics::InternalPhysicConfig::default())
                }
                "Plugin" => *self = PhysicsConfig::Plugin(crate::utils::GeneralValue::default()),
                _ => panic!("Where did you find this value?"),
            };
        }
        match self {
            PhysicsConfig::Internal(c) => c.show_mut(
                ui,
                ctx,
                buffer_stack,
                global_config,
                current_node_name,
                unique_id,
            ),
            PhysicsConfig::Plugin(c) => edit_plugin_config(
                ui,
                "Plugin Physics",
                "plugin-physics",
                unique_id,
                buffer_stack,
                c,
            ),
        }
    }

    fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str) {
        ui.horizontal(|ui| {
            ui.label(format!("Physics: {}", self));
        });
        match self {
            PhysicsConfig::Internal(c) => c.show(ui, ctx, unique_id),
            PhysicsConfig::Plugin(c) => show_plugin_config(ui, "Plugin Physics", c),
        }
    }
}

/// Enumeration of the records by physics implementations.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PhysicsRecord {
    /// Record emitted by internal physics.
    Internal(internal_physics::InternalPhysicsRecord),
    /// Record emitted by plugin physics.
    Plugin(crate::utils::GeneralValue),
}

impl PhysicsRecord {
    /// Returns the pose encoded in this record when available.
    ///
    /// For external and Python records, this currently returns `[0.0, 0.0, 0.0]`.
    pub fn pose(&self) -> [f32; 3] {
        match self {
            Self::Plugin(_) => [0., 0., 0.], // TODO: Find a way to get info from plugin record
            Self::Internal(p) => p.state.pose,
        }
    }
}

#[cfg(feature = "gui")]
impl UIComponent for PhysicsRecord {
    fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str) {
        ui.vertical(|ui| match self {
            Self::Internal(r) => {
                egui::CollapsingHeader::new("Internal").show(ui, |ui| {
                    r.show(ui, ctx, unique_id);
                });
            }
            Self::Plugin(r) => show_plugin_config(ui, "Plugin Physics", r),
        });
    }
}

use crate::{
    context::Context,
    errors::{SimusaError, SimusaErrorTypes, SimusaResult},
    node::{Node, node_factory::FromConfigArguments},
    physics::robot_models::Command,
    recordable::Recordable,
    state_estimators::State,
    utils::SharedRwLock,
};
#[cfg(feature = "gui")]
use crate::{
    gui::{
        UIComponent,
        utils::{edit_plugin_config, show_plugin_config, string_combobox},
    },
    simulator::SimulatorConfig,
    utils::enum_tools::ToVec,
};

// Services
#[derive(Debug, Clone)]
/// Service request for retrieving the real (ground-truth) state.
pub struct GetRealStateReq {}

#[derive(Debug, Clone)]
/// Service response containing the real (ground-truth) state.
pub struct GetRealStateResp {
    /// Ground-truth state.
    pub state: State,
}

/// Physics simulation trait.
pub trait Physics:
    std::fmt::Debug + std::marker::Send + std::marker::Sync + Recordable<PhysicsRecord>
{
    /// Optional initialization hook called once after node setup.
    #[allow(unused_variables)]
    fn post_init(&mut self, node: &mut Node, context: &Context) -> SimusaResult<()> {
        Ok(())
    }

    /// Apply the given `command` to the internal state from the last update time
    /// to the given `time`
    ///
    /// ## Arguments
    /// * `command` - Command to apply
    /// * `time` - Current time, when to apply the command.
    /// * `context` - Shared simulation context used for logging and communication metadata.
    fn apply_command(&mut self, command: &Command, time: f32, context: &Context);

    /// Update the state to the given time, while keeping the previous command.
    ///
    /// `context` can be used for logging or diagnostics during integration.
    fn update_state(&mut self, time: f32, context: &Context);

    /// Get the current real state, the groundtruth.
    ///
    /// `context` can be used for logging or diagnostics while retrieving the state.
    ///
    /// Optional: return None if the state is not available at the current time (e.g., if the node is terminated).
    fn state(&self, time: f32, context: &Context) -> Option<State>;

    /// Optional: return the time of the next time step. Needed if using messages
    #[allow(unused_variables)]
    fn next_time_step(&self, context: &Context) -> Option<f32> {
        None
    }
}

/// Helper function to create a physics from the given configuration.
///
/// ## Arguments
/// * `config` - Physics configuration.
/// * `from_config_args` - Additional constructor dependencies (node name, plugin API, global config, random-variable factory, network, and initial time).
/// * `context` - Shared simulation context used for logging and call tracing during construction.
pub fn make_physics_from_config(
    config: &PhysicsConfig,
    from_config_args: &FromConfigArguments,
) -> SimusaResult<SharedRwLock<Box<dyn Physics>>> {
    Ok(Arc::new(RwLock::new(match &config {
        PhysicsConfig::Internal(c) => Box::new(internal_physics::InternalPhysics::from_config(
            c,
            from_config_args,
        )?) as Box<dyn Physics>,
        PhysicsConfig::Plugin(c) => from_config_args
            .plugin_api
            .as_ref()
            .ok_or_else(|| {
                SimusaError::new(
                    SimusaErrorTypes::PluginAPIError,
                    "Plugin API not set!".to_string(),
                )
            })?
            .get_physics(
                c,
                from_config_args.global_config,
                from_config_args.va_factory,
                from_config_args.network,
                from_config_args.initial_time,
                from_config_args.context,
            ),
    })))
}
