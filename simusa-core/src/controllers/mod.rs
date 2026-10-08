//! Controller strategies and shared controller interfaces.
//!
//! This module defines the [`Controller`] trait used to convert navigation errors into robot
//! [`Command`] values consumed by [`Physics`](crate::physics::Physics).
//!
//! It also exposes strategy-specific configuration and runtime record enums:
//! [`ControllerConfig`] and [`ControllerRecord`].

pub mod pid;

use crate::{
    context::Context,
    errors::{SimusaError, SimusaErrorTypes, SimusaResult},
    node::node_factory::FromConfigArguments,
    physics::{PhysicsConfig, robot_models::Command},
    recordable::Recordable,
    utils::SharedRwLock,
};
#[cfg(feature = "gui")]
use crate::{
    gui::{
        UIComponent,
        utils::{edit_plugin_config, show_plugin_config, string_combobox},
    },
    utils::enum_tools::ToVec,
};
use std::sync::{Arc, RwLock};

use crate::simulator::SimulatorConfig;

use config_checker::*;
use serde_derive::{Deserialize, Serialize};
use simusa_macros::config_derives;

/// Errors used by the controllers: lateral, orientation and velocity.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ControllerError {
    /// Lateral error.
    pub lateral: f32,
    /// Longitudinal error.
    pub longitudinal: f32,
    /// Orientation error.
    pub theta: f32,
    /// Velocity error.
    pub velocity: f32,
}

#[cfg(feature = "gui")]
impl UIComponent for ControllerError {
    fn show(&self, ui: &mut egui::Ui, _ctx: &egui::Context, _unique_id: &str) {
        ui.vertical(|ui| {
            ui.label(format!("longitudinal: {}", self.longitudinal));
            ui.label(format!("lateral: {}", self.lateral));
            ui.label(format!("theta: {}", self.theta));
            ui.label(format!("velocity: {}", self.velocity));
        });
    }
}

/// Enumerates the strategies configurations.
#[config_derives]
pub enum ControllerConfig {
    #[check]
    /// Configuration for [`PID`](pid::PIDConfig).
    PID(pid::PIDConfig),
    /// Configuration for a plugin [`Controller`].
    Plugin(serde_json::Value),
}

#[cfg(feature = "gui")]
impl UIComponent for ControllerConfig {
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
            ui.label("Controller:");
            string_combobox(
                ui,
                &ControllerConfig::to_vec(),
                &mut current_str,
                format!("controller-choice-{}", unique_id),
            );
        });
        if current_str != self.to_string() {
            match current_str.as_str() {
                "PID" => *self = ControllerConfig::PID(pid::PIDConfig::default()),
                "Plugin" => *self = ControllerConfig::Plugin(serde_json::Value::Null),
                _ => panic!("Where did you find this value?"),
            };
        }
        match self {
            ControllerConfig::PID(c) => c.show_mut(
                ui,
                ctx,
                buffer_stack,
                global_config,
                current_node_name,
                unique_id,
            ),
            ControllerConfig::Plugin(c) => edit_plugin_config(
                ui,
                "Plugin Controller",
                "plugin-controller",
                unique_id,
                buffer_stack,
                c,
            ),
        }
    }

    fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str) {
        ui.horizontal(|ui| {
            ui.label(format!("Controller: {}", self));
        });
        match self {
            ControllerConfig::PID(c) => c.show(ui, ctx, unique_id),
            ControllerConfig::Plugin(c) => show_plugin_config(ui, "Plugin Controller", c),
        }
    }
}

/// Enumerates the strategies records.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ControllerRecord {
    /// Runtime record for [`pid::PID`].
    PID(pid::PIDRecord),
    /// Runtime record for a plugin [`Controller`].
    Plugin(serde_json::Value),
}

#[cfg(feature = "gui")]
impl UIComponent for ControllerRecord {
    fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str) {
        ui.vertical(|ui| match self {
            Self::PID(r) => {
                egui::CollapsingHeader::new("PID").show(ui, |ui| {
                    r.show(ui, ctx, unique_id);
                });
            }
            Self::Plugin(r) => show_plugin_config(ui, "Plugin Controller", r),
        });
    }
}

use crate::node::Node;

/// Controller strategy that computes [`Command`] values from control errors.
pub trait Controller:
    std::fmt::Debug + std::marker::Send + std::marker::Sync + Recordable<ControllerRecord>
{
    /// Performs optional one-time initialization when the node starts, before entering simulation loop.
    #[allow(unused_variables)]
    fn post_init(&mut self, node: &mut Node, context: &Context) -> SimusaResult<()> {
        Ok(())
    }

    /// Compute the command from the given error.
    ///
    /// ## Arguments
    /// * `robot` - Reference to the robot to access modules.
    /// * `error` - Error to be corrected.
    /// * `time` - Current time.
    /// * `context` - Shared simulation context used for logging and communication metadata.
    ///
    /// ## Return
    /// Command to apply to the [`Physics`](crate::physics::Physics).
    fn make_command(
        &mut self,
        robot: &mut Node,
        error: &ControllerError,
        time: f32,
        context: &Context,
    ) -> Command;

    /// Executes per-step side effects before command computation.
    fn pre_loop_hook(&mut self, node: &mut Node, time: f32, context: &Context);

    /// Optional: return the time of the next time step. Needed if using messages
    #[allow(unused_variables)]
    fn next_time_step(&self, context: &Context) -> Option<f32> {
        None
    }
}

/// Helper function to make the right [`Controller`] from the given configuration.
///
/// ## Arguments
/// * `config` - Configuration to use to make the controller.
/// * `plugin_api` - Optional PluginAPI to transmit to the controller.
/// * `global_config` - Global simulator configuration to transmit to the controller.
/// * `va_factory` - Random variables factory for determinist behavior.
/// * `physics_config` - Physics configuration to transmit to the controller and infer the robot model from.
/// * `network` - Shared reference to the network, for controllers using messages.
/// * `initial_time` - Initial node time.
/// * `context` - Shared simulation context used for logging and call tracing during construction.
pub fn make_controller_from_config(
    config: &ControllerConfig,
    from_config_params: &FromConfigArguments,
    physics_config: &PhysicsConfig,
) -> SimusaResult<SharedRwLock<Box<dyn Controller>>> {
    Ok(Arc::new(RwLock::new(match config {
        ControllerConfig::PID(c) => Box::new(pid::PID::from_config(
            c,
            physics_config,
            from_config_params.initial_time,
            from_config_params.context,
        )) as Box<dyn Controller>,
        ControllerConfig::Plugin(c) => from_config_params
            .plugin_api
            .as_ref()
            .ok_or_else(|| {
                SimusaError::new(
                    SimusaErrorTypes::PluginAPIError,
                    "Plugin API not set!".to_string(),
                )
            })?
            .get_controller(
                c,
                from_config_params.global_config,
                from_config_params.va_factory,
                from_config_params.network,
                from_config_params.initial_time,
                from_config_params.context,
            ),
    })))
}
