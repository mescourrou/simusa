//! Node configuration and factory utilities.
//!
//! This module defines node-type configurations/records and provides [`NodeFactory`]
//! helpers to instantiate runtime nodes from simulator configuration.

use std::{
    collections::BTreeMap,
    str::FromStr,
    sync::{Arc, RwLock},
};

use config_checker::*;
use serde::{Deserialize, Serialize};
use simusa_com::pub_sub::{BrokerTrait, PathKey};
use simusa_macros::{UIComponent, config_derives};

#[cfg(feature = "gui")]
use crate::gui::{UIComponent, utils::text_singleline_with_apply};

use crate::{
    api::internal_api::NodeServer,
    context::Context,
    controllers::{self, ControllerConfig, ControllerRecord, pid},
    environment::{Environment, map::MapRecord},
    errors::{SimusaError, SimusaErrorTypes, SimusaResult},
    interfaces::navigator_controller::ControllerInputRecord,
    internal,
    navigators::{self, NavigatorConfig, NavigatorRecord, go_to},
    networking::{
        self,
        network::{Network, NetworkConfig},
    },
    node::{Node, NodeMetaData, NodeState},
    physics::{self, PhysicsConfig, PhysicsRecord, internal_physics, robot_models::Command},
    plugin_api::PluginAPI,
    scenario::config::EventRecord,
    sensors::sensor_manager::{SensorManager, SensorManagerConfig, SensorManagerRecord},
    simulator::{SimulatorConfig, SimusaBroker, SimusaBrokerMultiClient},
    state_estimators::{
        self, BenchStateEstimator, BenchStateEstimatorConfig, BenchStateEstimatorRecord, State,
        StateEstimatorConfig, StateEstimatorRecord, WorldStateRecord, perfect_estimator,
    },
    time_analysis::{TimeAnalysisFactory, TimeAnalysisNode},
    utils::{
        SharedMutex, SharedRoLock, SharedRwLock,
        determinist_random_variable::DeterministRandomVariableFactory,
    },
};

/// Type of node instantiated in the simulator.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum NodeType {
    /// Full robot node with physics, control, navigation and sensing.
    Robot,
    /// Sensor-only node.
    Sensor,
    /// Passive object node.
    Object,
    /// Computation-only node.
    ComputationUnit,
}

impl NodeType {
    #[cfg(test)]
    const VALUES: [Self; 4] = [
        Self::Robot,
        Self::Sensor,
        Self::Object,
        Self::ComputationUnit,
    ];

    /// Returns whether this node type owns a physics module.
    pub fn has_physics(&self) -> bool {
        match self {
            Self::Robot | Self::Object => true,
            Self::Sensor | Self::ComputationUnit => false,
        }
    }

    /// Returns whether this node type owns a controller module.
    pub fn has_controller(&self) -> bool {
        match self {
            Self::Robot => true,
            Self::Object | Self::Sensor | Self::ComputationUnit => false,
        }
    }

    /// Returns whether this node type owns a navigator module.
    pub fn has_navigator(&self) -> bool {
        match self {
            Self::Robot => true,
            Self::Object | Self::Sensor | Self::ComputationUnit => false,
        }
    }

    /// Returns whether this node type owns a primary state estimator.
    pub fn has_state_estimator(&self) -> bool {
        match self {
            Self::Robot => true,
            Self::Object | Self::Sensor | Self::ComputationUnit => false,
        }
    }

    /// Returns whether this node type can host benchmark state estimators.
    pub fn has_state_estimator_bench(&self) -> bool {
        match self {
            Self::Robot | Self::Sensor | Self::ComputationUnit => true,
            Self::Object => false,
        }
    }

    /// Returns whether this node type owns a sensor manager.
    pub fn has_sensors(&self) -> bool {
        match self {
            Self::Robot | Self::Sensor | Self::ComputationUnit => true,
            Self::Object => false,
        }
    }

    /// Returns whether this node type owns a network module.
    pub fn has_network(&self) -> bool {
        match self {
            Self::Robot | Self::Sensor | Self::ComputationUnit => true,
            Self::Object => false,
        }
    }
}

/// Record enum for node runtime snapshots and other recorded elements
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RecordType {
    // Box RobotRecord to reduce size of NodeRecord
    /// Record payload for a robot node.
    Robot(Box<RobotRecord>),
    /// Record payload for a computation-unit node.
    ComputationUnit(Box<ComputationUnitRecord>),
    /// Scenario-based events records
    Scenario(Box<EventRecord>),
    /// Map generation record
    Map(Box<MapRecord>),
}

#[cfg(feature = "gui")]
impl UIComponent for RecordType {
    fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str) {
        match &self {
            Self::Robot(robot_record) => robot_record.show(ui, ctx, unique_id),
            Self::ComputationUnit(cu_record) => cu_record.show(ui, ctx, unique_id),
            Self::Scenario(record) => record.show(ui, ctx, unique_id),
            Self::Map(record) => record.show(ui, ctx, unique_id),
        }
    }
}

impl RecordType {
    /// Returns the corresponding [`NodeType`] for this record.
    pub fn as_node_type(&self) -> Option<NodeType> {
        match &self {
            Self::Robot(_) => Some(NodeType::Robot),
            Self::ComputationUnit(_) => Some(NodeType::ComputationUnit),
            _ => None,
        }
    }

    /// Returns the navigator record when available.
    pub fn navigator(&self) -> Option<&NavigatorRecord> {
        match &self {
            Self::Robot(robot_record) => robot_record.navigator.as_ref(),
            Self::ComputationUnit(_) => None,
            _ => None,
        }
    }

    /// Returns the controller record when available.
    pub fn controller(&self) -> Option<&ControllerRecord> {
        match &self {
            Self::Robot(robot_record) => robot_record.controller.as_ref(),
            Self::ComputationUnit(_) => None,
            _ => None,
        }
    }

    /// Returns the physics record when available.
    pub fn physics(&self) -> Option<&PhysicsRecord> {
        match &self {
            Self::Robot(robot_record) => Some(&robot_record.physics),
            Self::ComputationUnit(_) => None,
            _ => None,
        }
    }

    /// Returns the state-estimator record when available.
    pub fn state_estimator(&self) -> Option<&StateEstimatorRecord> {
        match &self {
            Self::Robot(robot_record) => robot_record.state_estimator.as_ref(),
            Self::ComputationUnit(_) => None,
            _ => None,
        }
    }

    /// Returns benchmark state-estimator records when available.
    pub fn state_estimator_bench(&self) -> Option<&Vec<BenchStateEstimatorRecord>> {
        match &self {
            Self::Robot(robot_record) => Some(&robot_record.state_estimator_bench),
            Self::ComputationUnit(computation_unit_record) => {
                Some(&computation_unit_record.state_estimators)
            }
            _ => None,
        }
    }

    /// Returns the sensor-manager record when available.
    pub fn sensor_manager(&self) -> Option<&SensorManagerRecord> {
        match &self {
            Self::Robot(robot_record) => Some(&robot_record.sensors),
            Self::ComputationUnit(r) => Some(&r.sensor_manager),
            _ => None,
        }
    }

    /// Returns node name.
    pub fn name(&self) -> Option<&String> {
        match &self {
            Self::Robot(robot_record) => Some(&robot_record.name),
            Self::ComputationUnit(r) => Some(&r.name),
            _ => None,
        }
    }
}

/// Root configuration for all nodes types.
#[config_derives]
#[derive(UIComponent)]
pub enum NodeTypeConfig {
    /// Configuration for [`NodeType::Robot`].
    #[check]
    Robot(Box<RobotConfig>),
    /// Configuration for [`NodeType::Sensor`].
    #[check]
    ComputationUnit(Box<ComputationUnitConfig>),
}

impl Default for NodeTypeConfig {
    fn default() -> Self {
        NodeTypeConfig::Robot(Box::default())
    }
}

impl NodeTypeConfig {
    /// Returns the corresponding [`NodeType`] for this configuration.
    pub fn as_node_type(&self) -> NodeType {
        match &self {
            Self::Robot(_) => NodeType::Robot,
            Self::ComputationUnit(_) => NodeType::ComputationUnit,
        }
    }
}

/// Full configuration of a [`Node`]
#[config_derives(skip_unknown_fields)] // skip_unknown_fields to be compatible with node_type flatten
pub struct NodeConfig {
    /// Name of the robot. Will be used in the `model_name` field as well.
    pub name: String,
    /// If `true`, node starts in running state when created.
    pub autospawn: bool,
    /// Free-form labels attached to the node metadata.
    pub labels: Vec<String>,
    /// Configuration of the type-specific behaviour
    #[serde(flatten)]
    pub node_type: NodeTypeConfig,
}

impl Default for NodeConfig {
    fn default() -> Self {
        NodeConfig {
            name: String::from("NoName"),
            autospawn: true,
            labels: Vec::new(),
            node_type: NodeTypeConfig::default(),
        }
    }
}

#[cfg(feature = "gui")]
impl UIComponent for NodeConfig {
    fn show_mut(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        buffer_stack: &mut std::collections::BTreeMap<String, String>,
        global_config: &SimulatorConfig,
        _current_node_name: Option<&String>,
        unique_id: &str,
    ) {
        let name_copy = self.name.clone();
        let current_node_name = Some(&name_copy);
        egui::CollapsingHeader::new(&self.name).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Name: ");
                text_singleline_with_apply(
                    ui,
                    format!("node-name-key-{}", unique_id).as_str(),
                    buffer_stack,
                    &mut self.name,
                );
            });

            ui.horizontal(|ui| {
                ui.label("Labels: ");

                let mut to_remove = Vec::new();
                for (i, label) in self.labels.iter_mut().enumerate() {
                    let unique_var_id = format!("node-labels-key-{}-{}", i, unique_id);
                    ui.horizontal(|ui| {
                        text_singleline_with_apply(ui, &unique_var_id, buffer_stack, label);
                    });
                    if ui.button("-").clicked() {
                        to_remove.push(i);
                    }
                }
                for i in to_remove.iter().rev() {
                    self.labels.remove(*i);
                }
                if ui.button("+").clicked() {
                    self.labels.push(String::new());
                }
            });

            ui.horizontal(|ui| {
                ui.label("Autospawn:");
                ui.checkbox(&mut self.autospawn, "");
            });

            self.node_type.show_mut(
                ui,
                ctx,
                buffer_stack,
                global_config,
                current_node_name,
                unique_id,
            );
        });
    }

    fn show(&self, ui: &mut egui::Ui, _ctx: &egui::Context, _unique_id: &str) {
        egui::CollapsingHeader::new(&self.name).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Name: {}", self.name));
            });

            ui.horizontal(|ui| {
                ui.label("Labels: ");
                ui.vertical(|ui| {
                    for label in &self.labels {
                        ui.label(format!("- '{}'", label));
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.label(format!("Autospawn: {}", self.autospawn));
            });
        });
    }
}

////////////////////////
/*        ROBOT       */
////////////////////////

/// Configuration of the [`NodeType::Robot`].
///
/// This configuration defines the different modules composing the robot, as well as their configuration.
///
/// Default values:
/// - `name`: `"NoName"`
/// - `navigator`: default [`go_to::GoToConfig`]
/// - `controller`: default [`pid::PIDConfig`]
/// - `physics`: default [`internal_physics::InternalPhysicConfig`]
/// - `state_estimator`: default [`perfect_estimator::PerfectEstimatorConfig`]
/// - `sensor_manager`: [`SensorManagerConfig::default`]
/// - `network`: [`NetworkConfig::default`]
/// - `state_estimator_bench`: empty vector
/// - `autospawn`: `true`
/// - `labels`: empty vector
///
/// # Example
/// ```yaml
/// nodes:
/// - name: robot2
///   type: Robot
///   navigator:
///     type: TrajectoryFollower
///     trajectory_path: path.yaml
///     forward_distance: 0.2
///     target_speed: 0.3
///     stop_distance: 0.2
///     stop_ramp_coefficient: 0.5
///   controller:
///     type: PID
///   physics:
///     type: Internal
///     model:
///       type: Unicycle
///       wheel_distance: 0.25
///     initial_state:
///       random:
///         - type: Uniform
///           min:
///           - -5.0
///           - -5.0
///           - -3.1416
///           max:
///           - 5.0
///           - 5.0
///           - 3.1416
///       variable_order: [x, y, orientation]
///     faults: []
///   state_estimator:
///     type: Perfect
///     prediction_activation:
///       period: {type: Num, value: 0.1}
///     targets:
///     - self
///   sensor_manager:
///     sensors:
///     - name: RobotSensor
///       send_to:
///       - Central Unit
///       config:
///         type: Robot
///         detection_distance: 100.0
///         activation_time:
///           period: {type: Num, value: 0.1}
///         faults: []
///         filters:
///           - type: Range
///             variables: [x, y]
///             min_range: [0, -1]
///             max_range: [10, 1]
///   network:
///     range: 10.0
///     reception_delay: 0.1
///   state_estimator_bench: []
/// ```
#[config_derives]
pub struct RobotConfig {
    /// [`Navigator`](crate::navigators::Navigator) to use, and its configuration.
    #[check]
    pub navigator: NavigatorConfig,
    /// [`Controller`](crate::controllers::Controller) to use, and its configuration.
    #[check]
    pub controller: ControllerConfig,
    /// [`Physics`](crate::physics::Physics) to use, and its configuration.
    #[check]
    pub physics: PhysicsConfig,
    /// [`StateEstimator`](crate::state_estimators::StateEstimator) to use, and its configuration.
    #[check]
    pub state_estimator: StateEstimatorConfig,
    /// [`SensorManager`] configuration, which defines the [`Sensor`](crate::sensors::Sensor)s used.
    #[check]
    pub sensor_manager: SensorManagerConfig,
    /// [`Network`] configuration.
    #[check]
    pub network: NetworkConfig,

    /// Additional [`StateEstimator`](crate::state_estimators::StateEstimator) to be evaluated but without a feedback
    /// loop with the [`Navigator`](crate::navigators::Navigator)
    #[check]
    pub state_estimator_bench: Vec<BenchStateEstimatorConfig>,
}

impl Default for RobotConfig {
    /// Default configuration, using:
    /// * Default [`GoTo`](go_to::GoTo) navigator.
    /// * Default [`PID`](pid::PID) controller.
    /// * Default [`InternalPhysics`](internal_physics::InternalPhysics) physics.
    /// * Default [`PerfectEstimator`](perfect_estimator::PerfectEstimator) state estimator.
    /// * Default [`SensorManager`] config (no sensors).
    /// * Default [`Network`] config.
    fn default() -> Self {
        RobotConfig {
            navigator: NavigatorConfig::GoTo(go_to::GoToConfig::default()),
            controller: ControllerConfig::PID(pid::PIDConfig::default()),
            physics: PhysicsConfig::Internal(internal_physics::InternalPhysicConfig::default()),
            state_estimator: StateEstimatorConfig::Perfect(
                perfect_estimator::PerfectEstimatorConfig::default(),
            ),
            sensor_manager: SensorManagerConfig::default(),
            network: NetworkConfig::default(),
            state_estimator_bench: Vec::new(),
        }
    }
}

#[cfg(feature = "gui")]
impl UIComponent for RobotConfig {
    fn show_mut(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        buffer_stack: &mut std::collections::BTreeMap<String, String>,
        global_config: &SimulatorConfig,
        current_node_name: Option<&String>,
        unique_id: &str,
    ) {
        self.network.show_mut(
            ui,
            ctx,
            buffer_stack,
            global_config,
            current_node_name,
            unique_id,
        );
        self.navigator.show_mut(
            ui,
            ctx,
            buffer_stack,
            global_config,
            current_node_name,
            unique_id,
        );
        self.physics.show_mut(
            ui,
            ctx,
            buffer_stack,
            global_config,
            current_node_name,
            unique_id,
        );
        self.controller.show_mut(
            ui,
            ctx,
            buffer_stack,
            global_config,
            current_node_name,
            unique_id,
        );
        self.state_estimator.show_mut(
            ui,
            ctx,
            buffer_stack,
            global_config,
            current_node_name,
            unique_id,
        );

        ui.label("State estimator bench:");
        let mut seb_to_remove = None;
        for (i, seb) in self.state_estimator_bench.iter_mut().enumerate() {
            let seb_unique_id = format!("{}-{}", unique_id, seb.name);
            ui.horizontal_top(|ui| {
                seb.show_mut(
                    ui,
                    ctx,
                    buffer_stack,
                    global_config,
                    current_node_name,
                    &seb_unique_id,
                );
                if ui.button("X").clicked() {
                    seb_to_remove = Some(i);
                }
            });
        }
        if let Some(i) = seb_to_remove {
            self.state_estimator_bench.remove(i);
        }
        if ui.button("Add State Estimator benched").clicked() {
            self.state_estimator_bench
                .push(BenchStateEstimatorConfig::default());
        }
        self.sensor_manager.show_mut(
            ui,
            ctx,
            buffer_stack,
            global_config,
            current_node_name,
            unique_id,
        );
    }

    fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str) {
        self.network.show(ui, ctx, unique_id);
        self.navigator.show(ui, ctx, unique_id);
        self.physics.show(ui, ctx, unique_id);
        self.controller.show(ui, ctx, unique_id);
        self.state_estimator.show(ui, ctx, unique_id);

        ui.label("State estimator bench:");
        for seb in &self.state_estimator_bench {
            let seb_unique_id = format!("{}-{}", unique_id, seb.name);
            ui.horizontal_top(|ui| {
                seb.show(ui, ctx, &seb_unique_id);
            });
        }

        self.sensor_manager.show(ui, ctx, unique_id);
    }
}

/// State record of [`NodeType::Robot`].
///
/// It contains the dynamic elements and the elements we want to save.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RobotRecord {
    /// Name of the robot.
    pub name: String,
    /// Config/model name used to instantiate the robot. It corresponds to the name of the robot
    /// in the configuration file.
    pub model_name: String,
    /// Record of the [`Navigator`](crate::navigators::Navigator) module if it was called.
    pub navigator: Option<NavigatorRecord>,
    /// Output of the [`Navigator`](crate::navigators::Navigator) if it was called.
    pub controller_input: Option<ControllerInputRecord>,
    /// Record of the [`Controller`](crate::controllers::Controller) module if it was called.
    pub controller: Option<ControllerRecord>,
    /// Output of the [`Controller`](crate::controllers::Controller) if it was called.
    pub command: Option<Command>,
    /// Record of the [`Physics`](crate::physics::Physics) module.
    pub physics: PhysicsRecord,
    /// Record of the [`StateEstimator`](crate::state_estimators::StateEstimator) module if prediction_step or correction_step was called.
    pub state_estimator: Option<StateEstimatorRecord>,
    /// Output of the loop [`StateEstimator`](crate::state_estimators::StateEstimator) if it was called.
    pub estimated_world_state: Option<WorldStateRecord>,
    /// Record of the additionnal [`StateEstimator`](crate::state_estimators::StateEstimator)s, only to evaluate them.
    pub state_estimator_bench: Vec<BenchStateEstimatorRecord>,

    /// Record of the [`SensorManager`] module.
    pub sensors: SensorManagerRecord,
    /// Current runtime node state.
    pub state: NodeState,
    /// Labels attached to the node.
    pub labels: Vec<String>,
}

#[cfg(feature = "gui")]
impl UIComponent for RobotRecord {
    fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str) {
        ui.vertical(|ui| {
            ui.label(format!("Name: {}", self.name));

            ui.label(format!("Model Name: {}", self.model_name));

            ui.label("Labels:");
            ui.vertical(|ui| {
                for label in &self.labels {
                    ui.label(format!("- '{}'", label));
                }
            });

            ui.label(format!("State: {}", self.state));

            if let Some(navigator) = &self.navigator {
                egui::CollapsingHeader::new("Navigator").show(ui, |ui| {
                    navigator.show(ui, ctx, unique_id);
                });
            } else {
                ui.label("Navigator was not called.");
            }

            if let Some(controller) = &self.controller {
                egui::CollapsingHeader::new("Controller").show(ui, |ui| {
                    controller.show(ui, ctx, unique_id);
                });
            } else {
                ui.label("Controller was not called.");
            }

            egui::CollapsingHeader::new("Physics").show(ui, |ui| {
                self.physics.show(ui, ctx, unique_id);
            });

            if let Some(state_estimator) = &self.state_estimator {
                egui::CollapsingHeader::new("State Estimator").show(ui, |ui| {
                    state_estimator.show(ui, ctx, unique_id);
                });
            } else {
                ui.label("State estimator was not called.");
            }

            ui.label("State Estimator bench:");
            for se in &self.state_estimator_bench {
                egui::CollapsingHeader::new(&se.name).show(ui, |ui| {
                    se.record.show(ui, ctx, unique_id);
                });
            }

            egui::CollapsingHeader::new("Sensors").show(ui, |ui| {
                self.sensors.show(ui, ctx, unique_id);
            });
        });
    }
}

////////////////////////
/* ComputationUnit    */
////////////////////////

/// Configuration of the [`NodeType::ComputationUnit`].
///
/// To get observations from other nodes, specify the `send_to` option of [`ManagedSensorConfig`](crate::sensors::sensor_manager::ManagedSensorConfig).
///
/// Default values:
/// - `name`: `"NoName"`
/// - `network`: [`NetworkConfig::default`]
/// - `state_estimators`: empty vector
/// - `labels`: empty vector
///
/// # Example
/// ```yaml
/// computation_units:
/// - name: Central Unit
///   state_estimators:
///   - name: central_perfect
///     config:
///       type: Perfect
///       prediction_activation:
///         period: {type: Num, value: 0.1}
///       targets:
///       - robot1
///       - robot2
/// ```
#[config_derives]
pub struct ComputationUnitConfig {
    /// [`Network`] configuration.
    #[check]
    pub network: NetworkConfig,
    /// [`StateEstimator`](crate::state_estimators::StateEstimator)s
    #[check]
    pub state_estimators: Vec<BenchStateEstimatorConfig>,
}

impl Default for ComputationUnitConfig {
    /// Default configuration, using:
    /// * Default [`Network`] config.
    fn default() -> Self {
        ComputationUnitConfig {
            network: NetworkConfig::default(),
            state_estimators: Vec::new(),
        }
    }
}

#[cfg(feature = "gui")]
impl UIComponent for ComputationUnitConfig {
    fn show_mut(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        buffer_stack: &mut std::collections::BTreeMap<String, String>,
        global_config: &SimulatorConfig,
        current_node_name: Option<&String>,
        unique_id: &str,
    ) {
        self.network.show_mut(
            ui,
            ctx,
            buffer_stack,
            global_config,
            current_node_name,
            unique_id,
        );

        ui.label("State estimators:");
        let mut se_to_remove = None;
        for (i, seb) in self.state_estimators.iter_mut().enumerate() {
            let seb_unique_id = format!("{}-{}", unique_id, seb.name);
            ui.horizontal_top(|ui| {
                seb.show_mut(
                    ui,
                    ctx,
                    buffer_stack,
                    global_config,
                    current_node_name,
                    &seb_unique_id,
                );
                if ui.button("X").clicked() {
                    se_to_remove = Some(i);
                }
            });
        }
        if let Some(i) = se_to_remove {
            self.state_estimators.remove(i);
        }
        if ui.button("Add State Estimator").clicked() {
            self.state_estimators
                .push(BenchStateEstimatorConfig::default());
        }
    }

    fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str) {
        self.network.show(ui, ctx, unique_id);

        ui.label("State estimators:");
        for seb in &self.state_estimators {
            let seb_unique_id = format!("{}-{}", unique_id, seb.name);
            ui.horizontal_top(|ui| {
                seb.show(ui, ctx, &seb_unique_id);
            });
        }
    }
}

/// State record of [`NodeType::ComputationUnit`].
///
/// It contains the dynamic elements and the elements we want to save.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComputationUnitRecord {
    /// Name of the robot.
    pub name: String,
    /// Records of benchmark state estimators.
    pub state_estimators: Vec<BenchStateEstimatorRecord>,
    /// Record of the [`SensorManager`] module.
    pub sensor_manager: SensorManagerRecord,
    /// Config/model name used to instantiate this unit.
    pub model_name: String,
    /// Labels attached to the node.
    pub labels: Vec<String>,
}

#[cfg(feature = "gui")]
impl UIComponent for ComputationUnitRecord {
    fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str) {
        ui.vertical(|ui| {
            ui.label(format!("Name: {}", self.name));

            ui.label(format!("Model Name: {}", self.model_name));

            ui.label("Labels:");
            ui.vertical(|ui| {
                for label in &self.labels {
                    ui.label(format!("- '{}'", label));
                }
            });

            ui.label("State Estimators:");
            for se in &self.state_estimators {
                egui::CollapsingHeader::new(&se.name).show(ui, |ui| {
                    se.record.show(ui, ctx, unique_id);
                });
            }

            egui::CollapsingHeader::new("Sensor Manager").show(ui, |ui| {
                self.sensor_manager.show(ui, ctx, unique_id);
            });
        });
    }
}

////////////////////////
/*      Factory       */
////////////////////////

/// Shared parameters used by node factory constructors.
pub struct MakeNodeParams<'a> {
    /// Optional plugin API bridge.
    pub plugin_api: &'a Option<Arc<dyn PluginAPI>>,
    /// Global simulator configuration.
    pub global_config: &'a SimulatorConfig,
    /// Deterministic random-variable factory.
    pub va_factory: &'a Arc<DeterministRandomVariableFactory>,
    /// Optional time-analysis factory used to create node profilers.
    pub time_analysis_factory: Option<&'a mut TimeAnalysisFactory>,
    /// If `true`, forces record transmission even without results configuration.
    pub force_send_results: bool,
    /// Optional override name for the created node.
    pub new_name: Option<&'a str>,
    /// Shared simulation message broker.
    pub broker: &'a SharedRwLock<SimusaBroker>,
    /// Initial simulation time.
    pub initial_time: f32,
    /// Shared simulation environment.
    pub environment: Arc<Environment>,
    /// Simulator context
    pub context: Context,
}

struct NodeBuilder {
    node_name: String,
    node_server: Option<NodeServer>,
    other_node_names: Vec<String>,
    time_analysis: Option<SharedMutex<TimeAnalysisNode>>,
    node_meta_data: SharedRwLock<NodeMetaData>,
    meta_data_list: Option<SharedRoLock<BTreeMap<String, SharedRoLock<NodeMetaData>>>>,
    node_message_client: SimusaBrokerMultiClient,
    environment: Arc<Environment>,
    send_record: bool,
}

impl NodeBuilder {
    fn make_global_channels(
        node_name: &String,
        broker: &SharedRwLock<SimusaBroker>,
        context: &Context,
    ) -> SimusaResult<SimusaBrokerMultiClient> {
        internal!(
            context,
            crate::logger::InternalLog::NetworkMessages,
            "Setup global channels for node '{}'",
            node_name
        );
        let mut broker_lock = broker.write().unwrap();
        broker_lock.add_channel(
            PathKey::from_str(networking::channels::internal::COMMAND)
                .unwrap()
                .join_str(node_name.as_str()),
        );
        let log_key = PathKey::from_str(networking::channels::internal::LOG)
            .unwrap()
            .join_str(node_name.as_str());
        broker_lock.add_channel(
            log_key
                .clone()
                .join_str(networking::channels::internal::log::ERROR),
        );
        broker_lock.add_channel(
            log_key
                .clone()
                .join_str(networking::channels::internal::log::WARNING),
        );
        broker_lock.add_channel(
            log_key
                .clone()
                .join_str(networking::channels::internal::log::INFO),
        );
        broker_lock.add_channel(
            log_key
                .clone()
                .join_str(networking::channels::internal::log::DEBUG),
        );
        internal!(
            context,
            crate::logger::InternalLog::NetworkMessages,
            "Broker channels:\n- {}",
            broker_lock
                .channel_list()
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<String>>()
                .join("\n- ")
        );
        std::mem::drop(broker_lock);
        let mut client = SimusaBrokerMultiClient::new(
            broker.clone(),
            node_name.clone(),
            0.,
            PathKey::from_str("/").unwrap(),
        );
        broker
            .write()
            .unwrap()
            .subscribe_to_list(
                &[PathKey::from_str(networking::channels::internal::COMMAND)
                    .unwrap()
                    .join_str(node_name.as_str())],
                0.,
                &mut client,
            )
            .unwrap();
        Ok(client)
    }

    pub fn from_config(config: &NodeConfig, params: &mut MakeNodeParams) -> SimusaResult<Node> {
        let node_name = config.name.clone();
        let node_type = config.node_type.as_node_type();
        let client = Self::make_global_channels(&node_name, params.broker, &params.context)?;
        let proto_node = Self {
            node_server: None,
            other_node_names: Vec::new(),
            time_analysis: params
                .time_analysis_factory
                .as_mut()
                .map(|taf| taf.new_node(node_name.clone())),
            node_meta_data: Arc::new(RwLock::new(NodeMetaData {
                name: node_name.clone(),
                node_type,
                model_name: node_name.clone(),
                labels: config.labels.clone(),
                state: if config.autospawn {
                    NodeState::Running
                } else {
                    NodeState::Created
                },
                physics: None,
            })),
            meta_data_list: None,
            node_message_client: client,
            environment: params.environment.clone(),
            node_name,
            send_record: params.force_send_results || params.global_config.results.is_some(),
        };
        match &config.node_type {
            NodeTypeConfig::Robot(robot_config) => proto_node.make_robot(robot_config, params),
            NodeTypeConfig::ComputationUnit(cu_config) => {
                proto_node.make_computation_unit(cu_config, params)
            }
        }
    }

    fn make_robot(self, config: &RobotConfig, params: &mut MakeNodeParams) -> SimusaResult<Node> {
        // Make global channels
        let network = Arc::new(RwLock::new(Network::from_config(
            self.node_name.clone(),
            &config.network,
            params.global_config,
            params.va_factory,
            params.broker,
            params.initial_time,
        )));
        let from_config_args = FromConfigArguments {
            global_config: params.global_config,
            initial_time: params.initial_time,
            network: &network,
            node_name: &self.node_name,
            plugin_api: params.plugin_api,
            va_factory: params.va_factory,
            context: &params.context,
        };
        let physics = physics::make_physics_from_config(&config.physics, &from_config_args)?;
        let initial_state = physics
            .read()
            .unwrap()
            .state(params.initial_time, &params.context)
            .unwrap()
            .clone();
        self.node_meta_data.write().unwrap().physics = Some(physics.clone());
        let node = Node {
            navigator: Some(navigators::make_navigator_from_config(
                &config.navigator,
                &from_config_args,
            )?),
            controller: Some(controllers::make_controller_from_config(
                &config.controller,
                &from_config_args,
                &config.physics,
            )?),
            physics: Some(physics),
            state_estimator: Some(Arc::new(RwLock::new(
                state_estimators::make_state_estimator_from_config(
                    &config.state_estimator,
                    &from_config_args,
                )?,
            ))),
            sensor_manager: Some(Arc::new(RwLock::new(SensorManager::from_config(
                &config.sensor_manager,
                &from_config_args,
                &initial_state,
                &params.context,
            )?))),
            network: Some(network.clone()),
            state_estimator_bench: Some(Arc::new(RwLock::new(Vec::with_capacity(
                config.state_estimator_bench.len(),
            )))),
            current_command: None,
            node_meta_data: self.node_meta_data,
            node_server: self.node_server,
            other_node_names: self.other_node_names,
            time_analysis: self.time_analysis,
            send_records: self.send_record,
            meta_data_list: self.meta_data_list,
            node_message_client: self.node_message_client,
            environment: self.environment,
            last_command: None,
            last_controller_input: None,
            last_estimation: None,
        };

        for state_estimator_config in &config.state_estimator_bench {
            node.state_estimator_bench
                .as_ref()
                .unwrap()
                .write()
                .unwrap()
                .push(BenchStateEstimator {
                    name: state_estimator_config.name.clone(),
                    state_estimator: Arc::new(RwLock::new(
                        state_estimators::make_state_estimator_from_config(
                            &state_estimator_config.config,
                            &from_config_args,
                        )?,
                    )),
                })
        }

        Ok(node)
    }

    fn make_computation_unit(
        self,
        config: &ComputationUnitConfig,
        params: &mut MakeNodeParams,
    ) -> SimusaResult<Node> {
        let network = Arc::new(RwLock::new(Network::from_config(
            self.node_name.clone(),
            &config.network,
            params.global_config,
            params.va_factory,
            params.broker,
            params.initial_time,
        )));
        let from_config_args = FromConfigArguments {
            global_config: params.global_config,
            initial_time: params.initial_time,
            network: &network,
            node_name: &self.node_name,
            plugin_api: params.plugin_api,
            va_factory: params.va_factory,
            context: &params.context,
        };
        let node = Node {
            navigator: None,
            controller: None,
            physics: None,
            state_estimator: None,
            sensor_manager: Some(Arc::new(RwLock::new(SensorManager::from_config(
                &SensorManagerConfig::default(),
                &from_config_args,
                &State::default(),
                &params.context,
            )?))),
            network: Some(network.clone()),
            state_estimator_bench: Some(Arc::new(RwLock::new(Vec::with_capacity(
                config.state_estimators.len(),
            )))),
            current_command: None,
            node_meta_data: self.node_meta_data,
            node_server: self.node_server,
            other_node_names: self.other_node_names,
            time_analysis: self.time_analysis,
            send_records: self.send_record,
            meta_data_list: self.meta_data_list,
            node_message_client: self.node_message_client,
            environment: self.environment,
            last_command: None,
            last_controller_input: None,
            last_estimation: None,
        };

        for state_estimator_config in &config.state_estimators {
            node.state_estimator_bench
                .as_ref()
                .unwrap()
                .write()
                .unwrap()
                .push(BenchStateEstimator {
                    name: state_estimator_config.name.clone(),
                    state_estimator: Arc::new(RwLock::new(
                        state_estimators::make_state_estimator_from_config(
                            &state_estimator_config.config,
                            &from_config_args,
                        )?,
                    )),
                })
        }

        Ok(node)
    }
}

/// Factory for creating runtime [`Node`] instances.
pub struct NodeFactory {}

impl NodeFactory {
    /// Builds a node from its configuration.
    pub fn make_node(config: &NodeConfig, params: &mut MakeNodeParams) -> SimusaResult<Node> {
        NodeBuilder::from_config(config, params)
    }

    /// Builds a node by searching its name in global robot and computation-unit configs.
    pub(crate) fn make_node_from_name(
        name: &str,
        params: &mut MakeNodeParams,
    ) -> SimusaResult<Node> {
        for node_config in params.global_config.nodes.iter() {
            if node_config.name == name {
                return NodeBuilder::from_config(node_config, params);
            }
        }

        Err(SimusaError::new(
            SimusaErrorTypes::ImplementationError,
            format!("Node `{}` unknown in configuration: cannot create", name),
        ))
    }
}

/// Arguments forwarded to subsystem `from_config` constructors during node creation.
pub struct FromConfigArguments<'a> {
    /// Optional plugin API bridge.
    pub plugin_api: &'a Option<Arc<dyn PluginAPI>>,
    /// Global simulator configuration.
    pub global_config: &'a SimulatorConfig,
    /// Name of the node being created.
    pub node_name: &'a String,
    /// Deterministic random-variable factory.
    pub va_factory: &'a Arc<DeterministRandomVariableFactory>,
    /// Node-local network handle.
    pub network: &'a SharedRwLock<Network>,
    /// Initial simulation time.
    pub initial_time: f32,
    /// Simulator context.
    pub context: &'a Context,
}

#[cfg(test)]
mod tests {

    mod node_type_rules {
        use super::super::NodeType;

        // The chain state_estimator -> navigator -> controller works together
        // The controller has no purpose if there is nothing to control => needs physics
        #[test]
        fn controller_needs_physics() {
            for node_type in NodeType::VALUES {
                if node_type.has_controller() {
                    assert!(node_type.has_physics());
                }
            }
        }

        #[test]
        fn controller_needs_navigator() {
            for node_type in NodeType::VALUES {
                if node_type.has_controller() {
                    assert!(node_type.has_navigator());
                }
            }
        }

        #[test]
        fn navigator_needs_state_estimator() {
            for node_type in NodeType::VALUES {
                if node_type.has_navigator() {
                    assert!(node_type.has_state_estimator());
                }
            }
        }
    }
}
