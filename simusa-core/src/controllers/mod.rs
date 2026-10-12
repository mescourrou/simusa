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
    interfaces::navigator_controller::{ControllerInputVariable, ControllerInputVariables},
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

/// Enumerates the strategies configurations.
#[config_derives]
pub enum ControllerConfig {
    #[check]
    /// Configuration for [`PID`](pid::PIDConfig).
    PID(pid::PIDConfig),
    /// Configuration for a plugin [`Controller`].
    Plugin(crate::utils::GeneralValue),
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
                "Plugin" => *self = ControllerConfig::Plugin(crate::utils::GeneralValue::default()),
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
    Plugin(crate::utils::GeneralValue),
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

/// Trait for the input of a [`Controller`], which can be created from [`ControllerInputVariables`].
///
/// Should not be implemented directly, but rather through the
/// [`declare_controller_input!`](crate::interfaces::navigator_controller::declare_controller_input) macro.
pub trait ControllerInput: From<ControllerInputVariables> + Sized {
    /// Returns the list of variables requested by the controller from the navigator.
    fn required_variables() -> &'static [ControllerInputVariable];
}

/// Controller strategy that computes [`Command`] values from control errors.
pub trait Controller:
    std::fmt::Debug + std::marker::Send + std::marker::Sync + Recordable<ControllerRecord>
{
    /// Returns the type of input expected by the controller, which can be converted from [`ControllerInputVariables`].
    type InputType: ControllerInput;

    /// Returns the list of variables requested by the controller from the navigator.
    ///
    /// Override with caution: the [`InputType`] might not be compatible with the requested variables.
    fn required_variables(&self) -> &[ControllerInputVariable] {
        Self::InputType::required_variables()
    }

    /// Performs optional one-time initialization when the node starts, before entering simulation loop.
    #[allow(unused_variables)]
    fn post_init(&mut self, node: &mut Node, context: &Context) -> SimusaResult<()> {
        Ok(())
    }

    /// Compute the command from the given input.
    ///
    /// ## Arguments
    /// * `robot` - Reference to the robot to access modules.
    /// * `input` - Input coming from the navigator, converted into [`ControllerInputVariables`].
    /// * `time` - Current time.
    /// * `context` - Shared simulation context used for logging and communication metadata.
    ///
    /// ## Return
    /// Command to apply to the [`Physics`](crate::physics::Physics).
    fn make_command(
        &mut self,
        robot: &mut Node,
        input: Self::InputType,
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

/// Trait for abstracting the [`Controller`] trait with dynamic conversion from
/// [`Navigator::OutputType`](crate::navigators::Navigator::OutputType) to
/// [`Controller::InputType`](crate::controllers::Controller::InputType), to be
/// used in the [`Node`] struct.
///
/// This trait is implemented automatically for any type that implements [`Controller`].
/// Should not be implemented directly!
pub trait AbstractController:
    std::fmt::Debug + std::marker::Send + std::marker::Sync + Recordable<ControllerRecord>
{
    /// Returns the list of variables requested by the controller from the navigator.
    fn call_required_variables(&self) -> &[ControllerInputVariable];

    /// Post-creation initialization of the controller, called at the end of the simulator creation.
    fn call_post_init(&mut self, node: &mut Node, context: &Context) -> SimusaResult<()>;

    /// Call of the hook at the beginning of the simulation loop, just after updating physics.
    fn call_pre_loop_hook(&mut self, node: &mut Node, time: f32, context: &Context);

    /// Compute the command from the given input.
    fn call_make_command(
        &mut self,
        robot: &mut Node,
        variables: ControllerInputVariables,
        time: f32,
        context: &Context,
    ) -> Command;

    /// Optional: return the time of the next time step. Needed if using messages
    fn call_next_time_step(&self, context: &Context) -> Option<f32>;
}

impl<T> AbstractController for T
where
    T: Controller,
{
    fn call_make_command(
        &mut self,
        robot: &mut Node,
        variables: ControllerInputVariables,
        time: f32,
        context: &Context,
    ) -> Command {
        self.make_command(robot, variables.into(), time, context)
    }

    fn call_required_variables(&self) -> &[ControllerInputVariable] {
        self.required_variables()
    }

    fn call_next_time_step(&self, context: &Context) -> Option<f32> {
        self.next_time_step(context)
    }

    fn call_post_init(&mut self, node: &mut Node, context: &Context) -> SimusaResult<()> {
        self.post_init(node, context)
    }

    fn call_pre_loop_hook(&mut self, node: &mut Node, time: f32, context: &Context) {
        self.pre_loop_hook(node, time, context)
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
) -> SimusaResult<SharedRwLock<Box<dyn AbstractController>>> {
    Ok(Arc::new(RwLock::new(match config {
        ControllerConfig::PID(c) => Box::new(pid::PID::from_config(
            c,
            physics_config,
            from_config_params.initial_time,
            from_config_params.context,
        )) as Box<dyn AbstractController>,
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
