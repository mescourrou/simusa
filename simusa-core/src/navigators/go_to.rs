//! Point-target [`Navigator`] implementation.
//!
//! [`GoTo`] drives the robot toward a single target point. The target can be configured at
//! startup through [`GoToConfig`] and updated at runtime using [`GoToMessage`] on
//! [`GoTo::CHANNEL_NAME`].
//!
//! The controller computes heading, longitudinal, lateral, and velocity errors from the current
//! ego state and the active target point.

use std::{
    str::FromStr,
    sync::{Arc, Mutex},
};

#[cfg(feature = "gui")]
use crate::{gui::UIComponent, simulator::SimulatorConfig};

use crate::{
    context::Context, errors::SimusaResult, info, navigators::{Navigator, NavigatorRecord}, networking::{message_types::MessageType, network::Network}, simulator::SimusaBrokerMultiClient, utils::{SharedMutex, SharedRwLock, geometry::smallest_theta_diff},
};
use crate::interfaces::navigator_controller::declare_navigator_output;

extern crate nalgebra as na;

use config_checker::*;
use nalgebra::SVector;
use serde_derive::{Deserialize, Serialize};
use simusa_com::pub_sub::{MultiClientTrait, PathKey};
use simusa_macros::config_derives;

#[config_derives]
#[derive(Default)]
/// Runtime message used to update the target point of a [`GoTo`] navigator.
pub struct GoToMessage {
    /// Target point in world coordinates `[x, y]`.
    ///
    /// `None` clears the current target.
    pub target_point: Option<[f32; 2]>,
}

/// Configuration of the [`GoTo`] strategy.
///
/// The target point can be set at startup through the `target_point` field, and updated at runtime by sending a [`GoToMessage`] on the [`GoTo::CHANNEL_NAME`] channel. If no target point is set, the navigator will not move the robot.
///
/// The target speed is reduced when the robot gets closer to the target point, starting from `stop_distance` meters to the target, using a ramp coefficient of `stop_ramp_coefficient`.
///
/// To reach the target point, the navigator computes the error to the target point in the robot frame:
/// cartesian error and angle error.
#[config_derives]
pub struct GoToConfig {
    /// Initial target point in world coordinates `[x, y]`.
    ///
    /// Default: `None`.
    pub target_point: Option<[f32; 2]>,
    /// Target linear speed in m/s.
    ///
    /// Default: `0.5`.
    pub target_speed: f32,
    /// Distance threshold to force stop near the target point, in meters.
    ///
    /// Default: `0.2`.
    pub stop_distance: f32,
    /// Ramp coefficient applied to remaining distance when reducing speed near the target.
    ///
    /// Default: `0.5`.
    pub stop_ramp_coefficient: f32,
}

impl Check for GoToConfig {
    fn do_check(&self) -> CheckResult {
        let mut ret = CheckResult::new();
        if self.target_speed < 0. {
            ret.add_error("Target speed should be positive".to_string());
        }
        if self.stop_distance < 0. {
            ret.add_error("Stop distance should be positive".to_string());
        }
        if self.stop_ramp_coefficient < 0. {
            ret.add_error("Stop ramp coefficient should be positive".to_string());
        }
        ret
    }
}

impl Default for GoToConfig {
    fn default() -> Self {
        Self {
            target_point: None,
            target_speed: 0.5,
            stop_distance: 0.2,
            stop_ramp_coefficient: 0.5,
        }
    }
}

#[cfg(feature = "gui")]
impl UIComponent for GoToConfig {
    fn show_mut(
        &mut self,
        ui: &mut egui::Ui,
        _ctx: &egui::Context,
        buffer_stack: &mut std::collections::BTreeMap<String, String>,
        _global_config: &SimulatorConfig,
        _current_node_name: Option<&String>,
        unique_id: &str,
    ) {
        egui::CollapsingHeader::new("Go To")
            .id_salt(format!("go-to-{}", unique_id))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    use crate::gui::utils::string_combobox;

                    ui.label("Target point: ".to_string());
                    let possible_values = vec!["None", "Some"];
                    let buffer_key = format!("go-to-target-point-option-{unique_id}");
                    if !buffer_stack.contains_key(&buffer_key) {
                        buffer_stack.insert(buffer_key.clone(), possible_values[0].to_string());
                    }
                    let value = buffer_stack.get_mut(&buffer_key).unwrap();
                    string_combobox(ui, &possible_values, value, format!("go-to-{}", unique_id));
                    if *value == possible_values[0] {
                        self.target_point = None;
                    } else {
                        use egui::DragValue;

                        if self.target_point.is_none() {
                            self.target_point = Some([0., 0.]);
                        }
                        let point = &mut self.target_point.unwrap();
                        ui.label("(".to_string());
                        ui.add(DragValue::new(&mut point[0]).fixed_decimals(2));
                        ui.label(", ".to_string());
                        ui.add(DragValue::new(&mut point[1]).fixed_decimals(2));
                        ui.label(")".to_string());
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Target speed:");
                    if self.target_speed < 0. {
                        self.target_speed = 0.;
                    }
                    ui.add(egui::DragValue::new(&mut self.target_speed).max_decimals(10));
                });

                ui.horizontal(|ui| {
                    ui.label("Stop distance:");
                    if self.stop_distance < 0. {
                        self.stop_distance = 0.;
                    }
                    ui.add(egui::DragValue::new(&mut self.stop_distance).max_decimals(10));
                });

                ui.horizontal(|ui| {
                    ui.label("Stop ramp coefficient:");
                    if self.stop_ramp_coefficient < 0. {
                        self.stop_ramp_coefficient = 0.;
                    }
                    ui.add(egui::DragValue::new(&mut self.stop_ramp_coefficient).max_decimals(10));
                });
            });
    }

    fn show(&self, ui: &mut egui::Ui, _ctx: &egui::Context, unique_id: &str) {
        egui::CollapsingHeader::new("Go To")
            .id_salt(format!("go-to-{}", unique_id))
            .show(ui, |ui| {
                if let Some(target_point) = &self.target_point {
                    ui.label(format!(
                        "Target point: ({}, {})",
                        target_point[0], target_point[1]
                    ));
                } else {
                    ui.label("Target point: None".to_string());
                }

                ui.horizontal(|ui| {
                    ui.label(format!("Target speed: {}", self.target_speed));
                });

                ui.horizontal(|ui| {
                    ui.label(format!("Stop distance: {}", self.stop_distance));
                });

                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Stop ramp coefficient: {}",
                        self.stop_ramp_coefficient
                    ));
                });
            });
    }
}

/// Record of the [`GoTo`].
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct GoToRecord {
    /// Current target point in world coordinates `[x, y]`.
    pub current_point: Option<[f32; 2]>,
}

#[cfg(feature = "gui")]
impl UIComponent for GoToRecord {
    fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str) {
        ui.vertical(|ui| {
            if let Some(current_point) = &self.current_point {
                ui.label(format!(
                    "Current point: ({}, {})",
                    current_point[0], current_point[1]
                ));
            } else {
                ui.label("Current point: None".to_string());
            }
        });
    }
}

/// [`Navigator`] strategy which goes to a given point. Can be received by message.
///
/// The lateral error is always 0.
#[derive(Debug)]
pub struct GoTo {
    /// Speed to reach in m/s
    target_speed: f32,
    current_point: Option<[f32; 2]>,
    /// Distance where to stop when reaching the end of the trajectory
    stop_distance: f32,
    /// Coefficient of the target velocity, multiplied by the remaining distance
    stop_ramp_coefficient: f32,

    message_client: SharedMutex<SimusaBrokerMultiClient>,
}

impl GoTo {
    /// Relative channel used to receive [`GoToMessage`] target updates. The full channel path use the
    /// [`NODE`](crate::networking::channels::internal::NODE) prefix and the node name, like `/simusa/nodes/<node_name>/navigator/goto`.
    pub const CHANNEL_NAME: &'static str = "navigator/goto";

    /// Makes a [`GoTo`] from the given config.
    ///
    /// ## Arguments
    /// * `config` - GoTo configuration
    /// * `network` - Shared reference to the network, used to subscribe to the target update messages on [`GoTo::CHANNEL_NAME`].
    /// * `initial_time` - Initial simulation time, in seconds. Not used by this navigator, but provided for consistency with other navigators and potential future use.
    pub fn from_config(
        config: &GoToConfig,
        network: &SharedRwLock<Network>,
        _initial_time: f32,
        context: &Context,
    ) -> Self {
        let network = network.write().unwrap();
        let key = network.make_channel(PathKey::from_str(Self::CHANNEL_NAME).unwrap(), context);
        let message_client = network.subscribe_to(&[key], None, context);
        Self {
            target_speed: config.target_speed,
            current_point: config.target_point,
            stop_distance: config.stop_distance,
            stop_ramp_coefficient: config.stop_ramp_coefficient,
            message_client: Arc::new(Mutex::new(message_client)),
        }
    }
}

use crate::node::Node;
use crate::state_estimators::WorldState;


declare_navigator_output! {
    #[derive(Default)]
    GoToOutput;
    node_position,
    node_orientation,
    node_linear_velocity,
    node_angular_velocity,
    node_longitudinal_lateral_velocities,
    target_position,
    target_orientation,
    target_linear_velocity,
    target_angular_velocity,
    target_longitudinal_lateral_velocities,
    error_longitudinal,
    error_lateral,
    error_angular,
    error_linear_velocity,
    error_angular_velocity,
    error_longitudinal_lateral_velocities,
    world_state,
}

impl Navigator for GoTo {
    type OutputType = GoToOutput;
    fn post_init(&mut self, _node: &mut Node, _context: &Context) -> SimusaResult<()> {
        Ok(())
    }

    /// Compute the error between the given `state` and the target point.
    ///
    fn compute_output(
        &mut self,
        _robot: &mut Node,
        world_state: WorldState,
        _context: &Context,
    ) -> SimusaResult<GoToOutput> {
        if world_state.ego.is_none() {
            panic!("StateEstimator should provide an ego estimate for GoTo navigator.")
        }

        let state = world_state.ego.as_ref().unwrap().clone().theta_modulo();

        let mut output = GoToOutput {
            world_state: world_state.clone(),
            node_position: (state.pose.x, state.pose.y),
            node_orientation: state.pose.z,
            node_linear_velocity: state.velocity.fixed_rows::<2>(0).norm(),
            node_angular_velocity: state.velocity[2],
            node_longitudinal_lateral_velocities: (state.velocity[0], state.velocity[1]),
            ..Default::default()
        };

        if self.current_point.is_none() {
            output.error_longitudinal = 0.;
            output.error_lateral = 0.;
            output.error_angular = 0.;
            output.error_linear_velocity = -output.node_linear_velocity;
            output.error_angular_velocity = -output.node_angular_velocity;
            output.error_longitudinal_lateral_velocities = (-output.node_longitudinal_lateral_velocities.0, -output.node_longitudinal_lateral_velocities.1);
            
            output.target_position = output.node_position;
            output.target_orientation = output.node_orientation;
            output.target_linear_velocity = 0.;
            output.target_angular_velocity = 0.;
            output.target_longitudinal_lateral_velocities = (0., 0.);
            return Ok(output);
        }
        let target_point = SVector::from_row_slice(&self.current_point.unwrap());
        let distance_to_final = (target_point - state.pose.fixed_view::<2, 1>(0, 0)).norm();
        let mut actual_target_speed = self
            .target_speed
            .min(distance_to_final * self.stop_ramp_coefficient);

        if distance_to_final < self.stop_distance {
            actual_target_speed = 0.;
        }

        let target_direction = (target_point[1] - state.pose.y).atan2(target_point[0] - state.pose.x);

        let theta_error = smallest_theta_diff(target_direction, state.pose.z);


        // Compute longitudinal and lateral errors
        // Need to project the target point in the robot frame
        let rot = na::Rotation2::new(-state.pose.z);
        let relative_target = rot * (target_point - state.pose.fixed_view::<2, 1>(0, 0));
        
        output.error_longitudinal = relative_target[0];
        output.error_lateral = relative_target[1];
        output.error_angular = theta_error;
        // Compute the velocity error
        output.error_linear_velocity = actual_target_speed - state.velocity.fixed_rows::<2>(0).norm();
        output.error_angular_velocity = 0. - state.velocity[2];
        output.error_longitudinal_lateral_velocities = (0., 0.); // Any velocity vector is acceptable, so we set the error to 0.

        output.target_position = (target_point[0], target_point[1]);
        output.target_orientation = target_direction;
        output.target_linear_velocity = actual_target_speed;
        output.target_angular_velocity = 0.;
        output.target_longitudinal_lateral_velocities = (actual_target_speed * relative_target[0] / distance_to_final, actual_target_speed * relative_target[1] / distance_to_final);

        Ok(output)
    }

    fn pre_loop_hook(&mut self, _node: &mut Node, time: f32, context: &Context) {
        while let Some((_, envelope)) = self.message_client.lock().unwrap().try_receive(time) {
            if let MessageType::GoTo(msg) = envelope.message {
                self.current_point = msg.target_point;
                info!(context, "Update target point to {:?}", self.current_point);
            }
        }
    }

    fn next_time_step(&self, _context: &Context) -> Option<f32> {
        self.message_client.lock().unwrap().next_message_time()
    }
}

use crate::recordable::Recordable;

impl Recordable<NavigatorRecord> for GoTo {
    fn record(&self, _context: &Context) -> NavigatorRecord {
        NavigatorRecord::GoTo(GoToRecord {
            current_point: self.current_point,
        })
    }
}
