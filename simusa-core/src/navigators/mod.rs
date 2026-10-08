//! Navigation strategies and shared navigation interfaces.
//!
//! This module defines the [`Navigator`] trait used by nodes to compute control errors from a
//! target behavior. It also exposes strategy-specific configuration and record enums:
//! [`NavigatorConfig`] and [`NavigatorRecord`].
//!
//! Built-in strategies include trajectory following, point-target navigation, and external/Python
//! implementations.

pub mod go_to;
pub mod trajectory;
pub mod trajectory_follower;

use std::sync::{Arc, RwLock};

use config_checker::*;
use serde_derive::{Deserialize, Serialize};
use simusa_macros::config_derives;

use crate::context::Context;
use crate::controllers::ControllerError;
use crate::errors::{SimusaError, SimusaErrorTypes, SimusaResult};
#[cfg(feature = "gui")]
use crate::gui::utils::{edit_plugin_config, show_plugin_config};
#[cfg(feature = "gui")]
use crate::gui::{UIComponent, utils::string_combobox};
use crate::node::node_factory::FromConfigArguments;
use crate::simulator::SimulatorConfig;
use crate::state_estimators::WorldState;

/// Enumerate the configuration of the different strategies.
///
/// The navigator computes control errors from a target behavior.
#[config_derives]
pub enum NavigatorConfig {
    /// Configuration for [`trajectory_follower::TrajectoryFollower`].
    #[check]
    TrajectoryFollower(trajectory_follower::TrajectoryFollowerConfig),
    /// Configuration for a plugin [`Navigator`].
    Plugin(serde_json::Value),
    /// Configuration for [`go_to::GoTo`].
    #[check]
    GoTo(go_to::GoToConfig),
}

#[cfg(feature = "gui")]
impl UIComponent for NavigatorConfig {
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
            ui.label("Navigator:");
            string_combobox(
                ui,
                &NavigatorConfig::to_vec(),
                &mut current_str,
                format!("navigator-choice-{}", unique_id),
            );
        });
        if current_str != self.to_string() {
            match current_str.as_str() {
                "TrajectoryFollower" => {
                    *self = NavigatorConfig::TrajectoryFollower(
                        trajectory_follower::TrajectoryFollowerConfig::default(),
                    )
                }
                "Plugin" => *self = NavigatorConfig::Plugin(serde_json::Value::Null),
                "GoTo" => *self = NavigatorConfig::GoTo(go_to::GoToConfig::default()),
                _ => panic!("Where did you find this value?"),
            };
        }
        match self {
            NavigatorConfig::TrajectoryFollower(c) => c.show_mut(
                ui,
                ctx,
                buffer_stack,
                global_config,
                current_node_name,
                unique_id,
            ),
            NavigatorConfig::Plugin(c) => edit_plugin_config(
                ui,
                "Plugin Navigator",
                "plugin-navigator",
                unique_id,
                buffer_stack,
                c,
            ),
            NavigatorConfig::GoTo(c) => c.show_mut(
                ui,
                ctx,
                buffer_stack,
                global_config,
                current_node_name,
                unique_id,
            ),
        }
    }

    fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str) {
        ui.horizontal(|ui| {
            ui.label(format!("Navigator: {}", self));
        });

        match self {
            NavigatorConfig::TrajectoryFollower(c) => c.show(ui, ctx, unique_id),
            NavigatorConfig::Plugin(c) => show_plugin_config(ui, "Plugin Navigator", c),
            NavigatorConfig::GoTo(c) => c.show(ui, ctx, unique_id),
        }
    }
}

/// Enumeration of the record of the different strategies.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NavigatorRecord {
    /// Runtime record for [`TrajectoryFollower`](trajectory_follower::TrajectoryFollower).
    TrajectoryFollower(trajectory_follower::TrajectoryFollowerRecord),
    /// Runtime record for a Plugin [`Navigator`].
    Plugin(serde_json::Value),
    /// Runtime record for [`GoTo`](go_to::GoTo).
    GoTo(go_to::GoToRecord),
}

#[cfg(feature = "gui")]
impl UIComponent for NavigatorRecord {
    fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str) {
        ui.vertical(|ui| match self {
            Self::TrajectoryFollower(r) => {
                egui::CollapsingHeader::new("TrajectoryFollower").show(ui, |ui| {
                    r.show(ui, ctx, unique_id);
                });
            }
            Self::Plugin(r) => show_plugin_config(ui, "Plugin Navigator", r),
            Self::GoTo(r) => {
                egui::CollapsingHeader::new("GoTo").show(ui, |ui| {
                    r.show(ui, ctx, unique_id);
                });
            }
        });
    }
}

use crate::node::Node;
use crate::recordable::Recordable;
use crate::utils::SharedRwLock;
#[cfg(feature = "gui")]
use crate::utils::enum_tools::ToVec;

/// Trait managing the planning, and providing the error to the planned behavior.
pub trait Navigator:
    std::fmt::Debug + std::marker::Send + std::marker::Sync + Recordable<NavigatorRecord>
{
    /// Performs optional one-time initialization when the node starts.
    #[allow(unused_variables)]
    fn post_init(&mut self, node: &mut Node, context: &Context) -> SimusaResult<()> {
        Ok(())
    }

    /// Compute the error ([`ControllerError`]) between the given `state` and the planned path.
    ///
    /// ## Arguments
    /// * `node` - Reference to the node to access modules.
    /// * `state` - Estimated world state used by the navigator.
    /// * `context` - Shared simulation context used for logging and communication metadata.
    fn compute_error(
        &mut self,
        node: &mut Node,
        state: WorldState,
        context: &Context,
    ) -> ControllerError;

    /// Executes per-step side effects before controller computation.
    fn pre_loop_hook(&mut self, node: &mut Node, time: f32, context: &Context);

    /// Optional: return the time of the next time step. Needed if using messages
    #[allow(unused_variables)]
    fn next_time_step(&self, context: &Context) -> Option<f32> {
        None
    }
}

/// Helper function to create a navigator from the given configuration.
///
/// ## Arguments
/// * `config` - Navigator configuration.
/// * `plugin_api` - Optional plugin API used by external/python navigators.
/// * `global_config` - Global simulator configuration.
/// * `va_factory` - Random variables factory for deterministic behavior.
/// * `network` - Shared reference to the network, for navigators using messages.
/// * `initial_time` - Initial node time.
/// * `context` - Shared simulation context used for logging and call tracing during construction.
pub fn make_navigator_from_config(
    config: &NavigatorConfig,
    from_config_params: &FromConfigArguments,
) -> SimusaResult<SharedRwLock<Box<dyn Navigator>>> {
    Ok(Arc::new(RwLock::new(match config {
        NavigatorConfig::TrajectoryFollower(c) => {
            Box::new(trajectory_follower::TrajectoryFollower::from_config(
                c,
                from_config_params.global_config,
                from_config_params.initial_time,
            )) as Box<dyn Navigator>
        }
        NavigatorConfig::Plugin(c) => from_config_params
            .plugin_api
            .as_ref()
            .ok_or_else(|| {
                SimusaError::new(
                    SimusaErrorTypes::PluginAPIError,
                    "Plugin API not set!".to_string(),
                )
            })?
            .get_navigator(
                c,
                from_config_params.global_config,
                from_config_params.va_factory,
                from_config_params.network,
                from_config_params.initial_time,
                from_config_params.context,
            ),
        NavigatorConfig::GoTo(c) => Box::new(go_to::GoTo::from_config(
            c,
            from_config_params.network,
            from_config_params.initial_time,
            from_config_params.context,
        )) as Box<dyn Navigator>,
    })))
}
