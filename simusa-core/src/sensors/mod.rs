//! Sensors module.
//!
//! This module defines the common sensor abstractions, shared observation/record types,
//! and configuration enums used by all sensor implementations.
//! Sensor instances are orchestrated by
//! [`SensorManager`](crate::sensors::sensor_manager::SensorManager), while each concrete sensor
//! implements [`Sensor`].
//!
//! To add a new sensor type:
//! 1. Implement [`Sensor`]
//! 2. Add a corresponding variant to [`SensorConfig`]
//! 3. Add a corresponding variant to [`SensorRecord`]

pub mod displacement_sensor;
pub mod gnss_sensor;
pub mod oriented_landmark_sensor;
pub mod robot_sensor;
pub mod scan_sensor;
pub mod sensor_manager;
pub mod speed_sensor;

pub mod fault_models;
pub mod sensor_filters;
use std::fmt::Debug;

use serde_derive::{Deserialize, Serialize};
use simusa_macros::config_derives;

use {
    gnss_sensor::{GNSSObservation, GNSSObservationRecord},
    oriented_landmark_sensor::{OrientedLandmarkObservation, OrientedLandmarkObservationRecord},
    robot_sensor::{OrientedRobotObservation, OrientedRobotObservationRecord},
    speed_sensor::{SpeedObservation, SpeedObservationRecord},
};

use crate::{
    context::Context,
    errors::SimusaResult,
    node::Node,
    recordable::Recordable,
    sensors::{
        displacement_sensor::{DisplacementObservation, DisplacementObservationRecord},
        scan_sensor::{ScanObservation, ScanObservationRecord},
    },
    utils::frame::{Frame, FrameRecord},
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

/// Runtime sensor observation with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    /// Sensor name that produced the observation.
    pub sensor_name: String,
    /// Name of the observing node.
    pub observer: String,
    /// Simulation time at which the observation was generated.
    pub time: f32,
    /// Sensor-specific observation payload.
    pub sensor_observation: SensorObservation,
    /// Frame of the sensor's pose observations, relative to the node's reference point.
    pub frame: Frame,
}

impl Observation {
    /// Creates a placeholder observation with default values.
    pub fn new() -> Self {
        Self {
            sensor_name: "sensor".to_string(),
            observer: "someone".to_string(),
            time: 0.,
            sensor_observation: SensorObservation::Speed(SpeedObservation::default()),
            frame: Frame::default(),
        }
    }

    /// Converts a [`ObservationRecord`] into a [`Observation`].
    pub fn from_record(record: &ObservationRecord) -> Self {
        Self {
            sensor_name: record.sensor_name.clone(),
            observer: record.observer.clone(),
            time: record.time,
            sensor_observation: SensorObservation::from_record(&record.sensor_observation),
            frame: Frame::from_record(&record.frame),
        }
    }
}

impl Default for Observation {
    fn default() -> Self {
        Self::new()
    }
}

impl Recordable<ObservationRecord> for Observation {
    fn record(&self, context: &Context) -> ObservationRecord {
        ObservationRecord {
            sensor_name: self.sensor_name.clone(),
            observer: self.observer.clone(),
            time: self.time,
            sensor_observation: self.sensor_observation.record(context),
            frame: self.frame.record(context),
        }
    }
}

/// Serializable record representation of [`Observation`].
#[config_derives(skip_partial_eq)]
#[derive(Default)]
pub struct ObservationRecord {
    /// Sensor name that produced the observation.
    pub sensor_name: String,
    /// Name of the observing node.
    pub observer: String,
    /// Simulation time at which the observation was generated.
    pub time: f32,
    /// Sensor-specific recorded payload.
    pub sensor_observation: SensorObservationRecord,
    /// Frame of the sensor's pose observations, relative to the node's reference point.
    pub frame: FrameRecord,
}

// Implementation of traits needed for sorting observations, first by time, then by sensor name, then by observer.
impl PartialEq for ObservationRecord {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
            && self.sensor_name == other.sensor_name
            && self.observer == other.observer
    }
}

impl Eq for ObservationRecord {}

impl PartialOrd for ObservationRecord {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for ObservationRecord {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.time.partial_cmp(&other.time) {
            Some(std::cmp::Ordering::Equal) => match self.sensor_name.cmp(&other.sensor_name) {
                std::cmp::Ordering::Equal => self.observer.cmp(&other.observer),
                other => other,
            },
            other => other.unwrap_or(std::cmp::Ordering::Equal),
        }
    }
}

#[cfg(feature = "gui")]
impl UIComponent for ObservationRecord {
    fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str) {
        ui.label(format!("Sensor name: {}", self.sensor_name));
        ui.label(format!("Observer: {}", self.observer));
        ui.label(format!("Time: {}", self.time));
        self.frame.show(ui, ctx, unique_id);
        self.sensor_observation.show(ui, ctx, unique_id);
    }
}

/// Sum type for all concrete sensor observation payloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SensorObservation {
    /// Oriented-landmark sensor observation payload.
    OrientedLandmark(OrientedLandmarkObservation),
    /// Speed sensor observation payload.
    Speed(SpeedObservation),
    /// Displacement sensor observation payload.
    Displacement(DisplacementObservation),
    /// GNSS sensor observation payload.
    GNSS(GNSSObservation),
    /// Robot sensor observation payload.
    OrientedRobot(OrientedRobotObservation),
    /// Scan sensor observation payload.
    Scan(ScanObservation),
    /// Plugin sensor observation payload.
    Plugin(serde_json::Value),
}

impl SensorObservation {
    /// Converts a [`SensorObservationRecord`] into a [`SensorObservation`].
    pub fn from_record(record: &SensorObservationRecord) -> Self {
        match record {
            SensorObservationRecord::OrientedLandmark(r) => {
                SensorObservation::OrientedLandmark(OrientedLandmarkObservation::from_record(r))
            }
            SensorObservationRecord::Speed(r) => {
                SensorObservation::Speed(SpeedObservation::from_record(r))
            }
            SensorObservationRecord::Displacement(r) => {
                SensorObservation::Displacement(DisplacementObservation::from_record(r))
            }
            SensorObservationRecord::GNSS(r) => {
                SensorObservation::GNSS(GNSSObservation::from_record(r))
            }
            SensorObservationRecord::OrientedRobot(r) => {
                SensorObservation::OrientedRobot(OrientedRobotObservation::from_record(r))
            }
            SensorObservationRecord::Scan(r) => {
                SensorObservation::Scan(ScanObservation::from_record(r))
            }
            SensorObservationRecord::Plugin(r) => SensorObservation::Plugin(r.clone()),
        }
    }
}

impl Recordable<SensorObservationRecord> for SensorObservation {
    fn record(&self, context: &Context) -> SensorObservationRecord {
        match self {
            SensorObservation::OrientedLandmark(o) => {
                SensorObservationRecord::OrientedLandmark(o.record(context))
            }
            SensorObservation::Speed(o) => SensorObservationRecord::Speed(o.record(context)),
            SensorObservation::Displacement(o) => {
                SensorObservationRecord::Displacement(o.record(context))
            }
            SensorObservation::GNSS(o) => SensorObservationRecord::GNSS(o.record(context)),
            SensorObservation::OrientedRobot(o) => {
                SensorObservationRecord::OrientedRobot(o.record(context))
            }
            SensorObservation::Scan(o) => SensorObservationRecord::Scan(o.record(context)),
            SensorObservation::Plugin(o) => SensorObservationRecord::Plugin(o.clone()),
        }
    }
}

/// Serializable record sum type for all sensor observations.
#[config_derives]
pub enum SensorObservationRecord {
    /// Record payload for oriented-landmark observations.
    OrientedLandmark(OrientedLandmarkObservationRecord),
    /// Record payload for speed observations.
    Speed(SpeedObservationRecord),
    /// Record payload for displacement observations.
    Displacement(DisplacementObservationRecord),
    /// Record payload for GNSS observations.
    GNSS(GNSSObservationRecord),
    /// Record payload for oriented-robot observations.
    OrientedRobot(OrientedRobotObservationRecord),
    /// Record payload for scan observations.
    Scan(ScanObservationRecord),
    /// Record payload for plugin observations.
    Plugin(serde_json::Value),
}

impl Default for SensorObservationRecord {
    fn default() -> Self {
        SensorObservationRecord::Speed(SpeedObservationRecord::default())
    }
}

#[cfg(feature = "gui")]
impl UIComponent for SensorObservationRecord {
    fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str) {
        ui.vertical(|ui| match self {
            Self::OrientedLandmark(r) => r.show(ui, ctx, unique_id),
            Self::Speed(r) => r.show(ui, ctx, unique_id),
            Self::Displacement(r) => r.show(ui, ctx, unique_id),
            Self::GNSS(r) => r.show(ui, ctx, unique_id),
            Self::OrientedRobot(r) => r.show(ui, ctx, unique_id),
            Self::Scan(r) => r.show(ui, ctx, unique_id),
            Self::Plugin(r) => show_plugin_config(ui, "Plugin Observation", r),
        });
    }
}

/// Enumerates all the possible sensors configurations.
///
/// Default value (from `#[config_derives]` generated implementation):
/// [`SensorConfig::OrientedLandmark`]
/// with [`OrientedLandmarkSensorConfig::default`](crate::sensors::oriented_landmark_sensor::OrientedLandmarkSensorConfig::default).
#[config_derives]
pub enum SensorConfig {
    /// Oriented-landmark sensor configuration.
    #[check]
    OrientedLandmark(oriented_landmark_sensor::OrientedLandmarkSensorConfig),
    /// Speed sensor configuration.
    #[check]
    Speed(speed_sensor::SpeedSensorConfig),
    /// Displacement sensor configuration.
    #[check]
    Displacement(displacement_sensor::DisplacementSensorConfig),
    /// GNSS sensor configuration.
    #[check]
    GNSS(gnss_sensor::GNSSSensorConfig),
    /// Robot sensor configuration.
    #[check]
    Robot(robot_sensor::RobotSensorConfig),
    /// Scan sensor configuration.
    #[check]
    Scan(scan_sensor::ScanSensorConfig),
    /// Plugin sensor configuration.
    Plugin(serde_json::Value),
}

#[cfg(feature = "gui")]
impl UIComponent for SensorConfig {
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
            ui.label("Sensor:");
            string_combobox(
                ui,
                &SensorConfig::to_vec(),
                &mut current_str,
                format!("sensor-choice-{}", unique_id),
            );
        });
        if current_str != self.to_string() {
            match current_str.as_str() {
                "OrientedLandmark" => {
                    *self = SensorConfig::OrientedLandmark(
                        oriented_landmark_sensor::OrientedLandmarkSensorConfig::default(),
                    )
                }
                "Speed" => *self = SensorConfig::Speed(speed_sensor::SpeedSensorConfig::default()),
                "Displacement" => {
                    *self = SensorConfig::Displacement(
                        displacement_sensor::DisplacementSensorConfig::default(),
                    )
                }
                "GNSS" => *self = SensorConfig::GNSS(gnss_sensor::GNSSSensorConfig::default()),
                "Robot" => *self = SensorConfig::Robot(robot_sensor::RobotSensorConfig::default()),
                "Scan" => *self = SensorConfig::Scan(scan_sensor::ScanSensorConfig::default()),
                "Plugin" => *self = SensorConfig::Plugin(serde_json::Value::Null),
                _ => panic!("Where did you find this value?"),
            };
        }
        match self {
            SensorConfig::OrientedLandmark(c) => c.show_mut(
                ui,
                ctx,
                buffer_stack,
                global_config,
                current_node_name,
                unique_id,
            ),
            SensorConfig::Speed(c) => c.show_mut(
                ui,
                ctx,
                buffer_stack,
                global_config,
                current_node_name,
                unique_id,
            ),
            SensorConfig::Displacement(c) => c.show_mut(
                ui,
                ctx,
                buffer_stack,
                global_config,
                current_node_name,
                unique_id,
            ),
            SensorConfig::GNSS(c) => c.show_mut(
                ui,
                ctx,
                buffer_stack,
                global_config,
                current_node_name,
                unique_id,
            ),
            SensorConfig::Robot(c) => c.show_mut(
                ui,
                ctx,
                buffer_stack,
                global_config,
                current_node_name,
                unique_id,
            ),
            SensorConfig::Scan(c) => c.show_mut(
                ui,
                ctx,
                buffer_stack,
                global_config,
                current_node_name,
                unique_id,
            ),
            SensorConfig::Plugin(c) => edit_plugin_config(
                ui,
                "Plugin Sensor",
                "plugin-sensor",
                unique_id,
                buffer_stack,
                c,
            ),
        }
    }

    fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str) {
        ui.horizontal(|ui| {
            ui.label(format!("Sensor: {}", self));
        });
        match self {
            SensorConfig::OrientedLandmark(c) => c.show(ui, ctx, unique_id),
            SensorConfig::Speed(c) => c.show(ui, ctx, unique_id),
            SensorConfig::Displacement(c) => c.show(ui, ctx, unique_id),
            SensorConfig::GNSS(c) => c.show(ui, ctx, unique_id),
            SensorConfig::Robot(c) => c.show(ui, ctx, unique_id),
            SensorConfig::Scan(c) => c.show(ui, ctx, unique_id),
            SensorConfig::Plugin(c) => show_plugin_config(ui, "Plugin Sensor", c),
        }
    }
}

/// Enumerates all the sensor records.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SensorRecord {
    /// Record produced by an oriented-landmark sensor.
    OrientedLandmarkSensor(oriented_landmark_sensor::OrientedLandmarkSensorRecord),
    /// Record produced by a speed sensor.
    SpeedSensor(speed_sensor::SpeedSensorRecord),
    /// Record produced by a displacement sensor.
    DisplacementSensor(displacement_sensor::DisplacementSensorRecord),
    /// Record produced by a GNSS sensor.
    GNSSSensor(gnss_sensor::GNSSSensorRecord),
    /// Record produced by a robot sensor.
    RobotSensor(robot_sensor::RobotSensorRecord),
    /// Record produced by a scan sensor.
    ScanSensor(scan_sensor::ScanSensorRecord),
    /// Record produced by a plugin sensor.
    Plugin(serde_json::Value),
}

#[cfg(feature = "gui")]
impl UIComponent for SensorRecord {
    fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str) {
        ui.vertical(|ui| match self {
            Self::OrientedLandmarkSensor(r) => {
                egui::CollapsingHeader::new("OrientedLandmark").show(ui, |ui| {
                    r.show(ui, ctx, unique_id);
                });
            }
            Self::SpeedSensor(r) => {
                egui::CollapsingHeader::new("Speed").show(ui, |ui| {
                    r.show(ui, ctx, unique_id);
                });
            }
            Self::DisplacementSensor(r) => {
                egui::CollapsingHeader::new("Displacement").show(ui, |ui| {
                    r.show(ui, ctx, unique_id);
                });
            }
            Self::GNSSSensor(r) => {
                egui::CollapsingHeader::new("GNSS").show(ui, |ui| {
                    r.show(ui, ctx, unique_id);
                });
            }
            Self::RobotSensor(r) => {
                egui::CollapsingHeader::new("Robot").show(ui, |ui| {
                    r.show(ui, ctx, unique_id);
                });
            }
            Self::ScanSensor(r) => {
                egui::CollapsingHeader::new("Scan").show(ui, |ui| {
                    r.show(ui, ctx, unique_id);
                });
            }
            Self::Plugin(r) => show_plugin_config(ui, "Plugin Sensor", r),
        });
    }
}

/// Sensor trait which need to be implemented by each sensors.
pub trait Sensor:
    std::fmt::Debug + std::marker::Send + std::marker::Sync + Recordable<SensorRecord>
{
    /// Initialize the [`Sensor`]. Should be called at the beginning of the run, after
    /// the initialization of the modules.
    #[allow(unused_variables)]
    fn post_init(
        &mut self,
        node: &mut Node,
        initial_time: f32,
        context: &Context,
    ) -> SimusaResult<()> {
        Ok(())
    }

    /// Get the observations available at the given `time`.
    ///
    /// ## Arguments
    /// * `node` - Reference to the node to access the modules.
    /// * `time` - Time at which the observations are taken.
    /// * `frame` - Position of the sensor relative to the node's reference point.
    /// * `context` - Shared simulation context used for logging and communication metadata.
    ///
    /// ## Return
    /// List of [`SensorObservation`]s, could be empty if no [`Sensor`] provided observation
    /// at this `time`.
    fn get_observations(
        &mut self,
        node: &mut Node,
        time: f32,
        frame: &Frame,
        context: &Context,
    ) -> Vec<SensorObservation>;

    /// Get the time of the next observation to trigger the next call to `get_observations`.
    /// This allows the sensor to have a custom observation period, or to trigger observations at specific times.
    fn next_time_step(&self) -> f32;
}
