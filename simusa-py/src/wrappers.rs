#![allow(clippy::useless_conversion)]
//! Python wrapper layer for Simusa core types exposed through PyO3.
//!
//! This module contains thin wrapper types used to:
//! - expose Rust data structures and APIs to Python,
//! - convert values between Python-facing wrappers and internal Rust models,
//! - provide Python entry points for simulation and networking.
//!
//! Most wrappers follow the same pattern:
//! - a `#[pyclass]` with Python-friendly fields,
//! - `from_rust` and `to_rust` conversion helpers,
//! - a default constructor for easy Python-side instantiation.

use std::{
    collections::BTreeMap,
    str::FromStr,
    sync::{Arc, RwLock, Weak},
};

use nalgebra::{SVector, Vector2, Vector3};
use pyo3::{exceptions::PyTypeError, prelude::*};
use simusa_com::pub_sub::{MultiClientTrait, PathKey};
use simusa_macros::EnumToString;

use simusa::{
    context::Context,
    debug, error, info,
    networking::network::{Envelope, Network},
    node::Node,
    physics::robot_models::{Command, holonomic::HolonomicCommand, unicycle::UnicycleCommand},
    plugin_api::PluginAPI,
    sensors::{
        Observation, SensorObservation, displacement_sensor::DisplacementObservation,
        gnss_sensor::GNSSObservation, oriented_landmark_sensor::OrientedLandmarkObservation,
        robot_sensor::OrientedRobotObservation, speed_sensor::SpeedObservation,
    },
    simulator::{AsyncSimulator, SimusaBrokerMultiClient},
    state_estimators::{State, WorldState},
    utils::{frame::Frame, occupancy_grid::OccupancyGrid},
    warning,
};

use crate::{
    controllers::ControllerWrapper,
    general::{PythonAPI, impl_from_plain_with_from_ref},
    messages::MessageFlagWrapper,
    navigators::NavigatorWrapper,
    networking::MessageTypeWrapper,
    physics::PhysicsWrapper,
    state_estimators::StateEstimatorWrapper,
};

#[pyclass(get_all, set_all)]
#[pyo3(name = "Pose")]
#[derive(Clone, Debug)]
/// 2D pose with heading angle used by Python APIs.
pub struct Pose {
    /// Position on x axis.
    pub x: f32,
    /// Position on y axis.
    pub y: f32,
    /// Heading angle (radians).
    pub theta: f32,
}

#[pyclass(get_all, set_all)]
#[pyo3(name = "Vec2")]
#[derive(Clone, Debug)]
/// Generic 2D vector used in Python wrappers.
pub struct Vec2 {
    /// x component.
    pub x: f32,
    /// y component.
    pub y: f32,
}

#[pyclass(get_all, set_all)]
#[pyo3(name = "Vec3")]
#[derive(Clone, Debug)]
/// Generic 3D vector used in Python wrappers.
pub struct Vec3 {
    /// x component.
    pub x: f32,
    /// y component.
    pub y: f32,
    /// z component.
    pub z: f32,
}

#[derive(Clone, Debug)]
#[pyclass(get_all, set_all)]
#[pyo3(name = "State")]
/// Python wrapper around a robot [`State`].
pub struct StateWrapper {
    /// Position and orientation of the robot
    pub pose: Pose,
    /// Linear velocity.
    pub velocity: Vec3,
}

#[pymethods]
impl StateWrapper {
    /// Create a zero-initialized state.
    #[new]
    pub fn new() -> Self {
        Self {
            pose: Pose {
                x: 0.,
                y: 0.,
                theta: 0.,
            },
            velocity: Vec3 {
                x: 0.,
                y: 0.,
                z: 0.,
            },
        }
    }
}

impl From<&State> for StateWrapper {
    /// Convert from the Rust [`State`] type.
    fn from(s: &State) -> Self {
        Self {
            pose: Pose {
                x: s.pose[0],
                y: s.pose[1],
                theta: s.pose[2],
            },
            velocity: Vec3 {
                x: s.velocity[0],
                y: s.velocity[1],
                z: s.velocity[2],
            },
        }
    }
}

impl_from_plain_with_from_ref!(State, StateWrapper);

impl From<&StateWrapper> for State {
    /// Convert this wrapper to the Rust [`State`] type.
    fn from(value: &StateWrapper) -> State {
        State {
            pose: SVector::from_vec(vec![value.pose.x, value.pose.y, value.pose.theta]),
            velocity: SVector::from_vec(vec![value.velocity.x, value.velocity.y, value.velocity.z]),
        }
    }
}
impl_from_plain_with_from_ref!(StateWrapper, State);

impl Default for StateWrapper {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
#[pyclass(get_all, set_all)]
#[pyo3(name = "WorldState")]
/// Python wrapper around a full [`WorldState`].
pub struct WorldStateWrapper {
    /// Optional state of the ego robot.
    pub ego: Option<StateWrapper>,
    /// States of named dynamic objects (other nodes).
    pub objects: BTreeMap<String, StateWrapper>,
    /// States of known landmarks indexed by id.
    pub landmarks: BTreeMap<i32, StateWrapper>,
    /// Optional occupancy grid map.
    pub occupancy_grid: Option<OccupancyGridWrapper>,
}

#[pymethods]
impl WorldStateWrapper {
    /// Create an empty world state.
    #[new]
    pub fn new() -> Self {
        Self {
            ego: None,
            objects: BTreeMap::new(),
            landmarks: BTreeMap::new(),
            occupancy_grid: None,
        }
    }
}

impl From<&WorldState> for WorldStateWrapper {
    /// Convert from the Rust [`WorldState`] type.
    fn from(s: &WorldState) -> Self {
        Self {
            ego: s.ego.as_ref().map(StateWrapper::from),
            landmarks: BTreeMap::from_iter(
                s.landmarks
                    .iter()
                    .map(|(id, s)| (*id, StateWrapper::from(s))),
            ),
            objects: BTreeMap::from_iter(
                s.objects
                    .iter()
                    .map(|(id, s)| (id.clone(), StateWrapper::from(s))),
            ),
            occupancy_grid: s.occupancy_grid.as_ref().map(OccupancyGridWrapper::from),
        }
    }
}
impl_from_plain_with_from_ref!(WorldState, WorldStateWrapper);

impl From<&WorldStateWrapper> for WorldState {
    /// Convert this wrapper to the Rust [`WorldState`] type.
    fn from(value: &WorldStateWrapper) -> WorldState {
        WorldState {
            ego: value.ego.as_ref().map(State::from),
            landmarks: BTreeMap::from_iter(
                value.landmarks.iter().map(|(id, s)| (*id, State::from(s))),
            ),
            objects: BTreeMap::from_iter(
                value
                    .objects
                    .iter()
                    .map(|(id, s)| (id.clone(), State::from(s))),
            ),
            occupancy_grid: value.occupancy_grid.as_ref().map(OccupancyGrid::from),
        }
    }
}
impl_from_plain_with_from_ref!(WorldStateWrapper, WorldState);

impl Default for WorldStateWrapper {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
#[pyclass]
#[pyo3(name = "OccupancyGrid")]
/// Python wrapper around [`OccupancyGrid`].
pub struct OccupancyGridWrapper {
    grid: OccupancyGrid,
}

#[pymethods]
impl OccupancyGridWrapper {
    /// Create a grid centered at `center` with fixed cell geometry.
    #[new]
    pub fn new(
        center: [f32; 3],
        cell_height: f32,
        cell_width: f32,
        nb_rows: usize,
        nb_cols: usize,
    ) -> Self {
        Self {
            grid: OccupancyGrid::new(
                Vector3::from(center),
                cell_height,
                cell_width,
                nb_rows,
                nb_cols,
            ),
        }
    }

    /// Get a cell value by `(row, col)` index.
    pub fn get_idx(&self, row: usize, col: usize) -> Option<f32> {
        self.grid.get_idx(row, col).cloned()
    }

    /// Set a cell value by `(row, col)` index.
    ///
    /// Returns `true` if the index exists, `false` otherwise.
    pub fn set_idx(&mut self, row: usize, col: usize, value: f32) -> bool {
        if let Some(v) = self.grid.get_idx_mut(row, col) {
            *v = value;
            true
        } else {
            false
        }
    }

    /// Get a cell value using world coordinates `[x, y]`.
    pub fn get_pos(&self, position: [f32; 2]) -> Option<f32> {
        self.grid.get_pos(Vector2::from(position)).cloned()
    }

    /// Set a cell value using world coordinates `[x, y]`.
    ///
    /// Returns `true` if the position maps to a valid cell, `false` otherwise.
    pub fn set_pos(&mut self, position: [f32; 2], value: f32) -> bool {
        if let Some(v) = self.grid.get_pos_mut(Vector2::from(position)) {
            *v = value;
            true
        } else {
            false
        }
    }
}

impl From<&OccupancyGrid> for OccupancyGridWrapper {
    /// Clone a Rust [`OccupancyGrid`] into its Python wrapper.
    fn from(s: &OccupancyGrid) -> Self {
        Self { grid: s.clone() }
    }
}
impl_from_plain_with_from_ref!(OccupancyGrid, OccupancyGridWrapper);

impl From<&OccupancyGridWrapper> for OccupancyGrid {
    /// Clone this wrapper back to a Rust [`OccupancyGrid`].
    fn from(value: &OccupancyGridWrapper) -> OccupancyGrid {
        value.grid.clone()
    }
}
impl_from_plain_with_from_ref!(OccupancyGridWrapper, OccupancyGrid);

#[derive(Clone, Debug)]
#[pyclass(get_all, set_all)]
#[pyo3(name = "OrientedLandmarkObservation")]
/// Python wrapper around an oriented landmark sensor observation.
pub struct OrientedLandmarkObservationWrapper {
    /// Id of the landmark
    pub id: i32,
    /// Labels of the landmark
    pub labels: Vec<String>,
    /// Relative pose of the landmark
    pub pose: Pose,
    /// Applied fault in JSON format
    pub applied_faults: String,
    /// Width of the landmark
    pub width: f32,
    /// Height of the landmark
    pub height: f32,
}

#[pymethods]
impl OrientedLandmarkObservationWrapper {
    /// Create a default-oriented landmark observation.
    #[new]
    pub fn new() -> Self {
        Self {
            id: 0,
            labels: Vec::new(),
            pose: Pose {
                x: 0.,
                y: 0.,
                theta: 0.,
            },
            width: 0.,
            height: 1.,
            applied_faults: "[]".to_string(),
        }
    }
}

impl From<&OrientedLandmarkObservation> for OrientedLandmarkObservationWrapper {
    /// Convert from the Rust [`OrientedLandmarkObservation`] type.
    fn from(s: &OrientedLandmarkObservation) -> Self {
        Self {
            id: s.id,
            labels: s.labels.clone(),
            pose: Pose {
                x: s.pose[0],
                y: s.pose[1],
                theta: s.pose[2],
            },
            applied_faults: serde_json::to_string(&s.applied_faults).unwrap(),
            width: s.width,
            height: s.height,
        }
    }
}
impl_from_plain_with_from_ref!(
    OrientedLandmarkObservation,
    OrientedLandmarkObservationWrapper
);

impl From<&OrientedLandmarkObservationWrapper> for OrientedLandmarkObservation {
    /// Convert this wrapper to the Rust [`OrientedLandmarkObservation`] type.
    fn from(value: &OrientedLandmarkObservationWrapper) -> OrientedLandmarkObservation {
        OrientedLandmarkObservation {
            id: value.id,
            labels: value.labels.clone(),
            pose: SVector::from_vec(vec![value.pose.x, value.pose.y, value.pose.theta]),
            applied_faults: serde_json::from_str(&value.applied_faults).unwrap(),
            width: value.width,
            height: value.height,
        }
    }
}
impl_from_plain_with_from_ref!(
    OrientedLandmarkObservationWrapper,
    OrientedLandmarkObservation
);

impl Default for OrientedLandmarkObservationWrapper {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
#[pyclass(get_all, set_all)]
#[pyo3(name = "SpeedObservation")]
/// Python wrapper around speed sensor observation values.
pub struct SpeedObservationWrapper {
    /// Linear velocity along robot forward axis.
    pub linear_velocity: f32,
    /// Linear velocity along robot lateral axis.
    pub lateral_velocity: f32,
    /// Angular velocity around vertical axis.
    pub angular_velocity: f32,
    /// Applied faults in JSON format
    pub applied_faults: String,
}

#[pymethods]
impl SpeedObservationWrapper {
    /// Create a default speed observation.
    #[new]
    pub fn new() -> Self {
        Self {
            linear_velocity: 0.,
            lateral_velocity: 0.,
            angular_velocity: 0.,
            applied_faults: "[]".to_string(),
        }
    }
}

impl From<&SpeedObservation> for SpeedObservationWrapper {
    /// Convert from the Rust [`SpeedObservation`] type.
    fn from(s: &SpeedObservation) -> Self {
        Self {
            linear_velocity: s.linear_velocity,
            lateral_velocity: s.lateral_velocity,
            angular_velocity: s.angular_velocity,
            applied_faults: serde_json::to_string(&s.applied_faults).unwrap(),
        }
    }
}
impl_from_plain_with_from_ref!(SpeedObservation, SpeedObservationWrapper);

impl From<&SpeedObservationWrapper> for SpeedObservation {
    /// Convert this wrapper to the Rust [`SpeedObservation`] type.
    fn from(value: &SpeedObservationWrapper) -> SpeedObservation {
        SpeedObservation {
            linear_velocity: value.linear_velocity,
            lateral_velocity: value.lateral_velocity,
            angular_velocity: value.angular_velocity,
            applied_faults: serde_json::from_str(&value.applied_faults).unwrap(),
        }
    }
}
impl_from_plain_with_from_ref!(SpeedObservationWrapper, SpeedObservation);

impl Default for SpeedObservationWrapper {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
#[pyclass(get_all)]
#[pyo3(name = "GNSSObservation")]
/// Python wrapper around GNSS-like pose and velocity observation.
pub struct GNSSObservationWrapper {
    /// Pose measurement `[x, y, theta]`.
    pub pose: Vec3,
    /// Velocity measurement `[vx, vy]`.
    pub velocity: Vec2,
    /// Applied faults in JSON format
    pub applied_faults: String,
}

#[pymethods]
impl GNSSObservationWrapper {
    /// Create a default GNSS observation.
    #[new]
    pub fn new() -> Self {
        Self {
            pose: Vec3 {
                x: 0.,
                y: 0.,
                z: 0.,
            },
            velocity: Vec2 { x: 0., y: 0. },
            applied_faults: "[]".to_string(),
        }
    }
}

impl From<&GNSSObservation> for GNSSObservationWrapper {
    /// Convert from the Rust [`GNSSObservation`] type.
    fn from(s: &GNSSObservation) -> Self {
        Self {
            pose: Vec3 {
                x: s.pose[0],
                y: s.pose[1],
                z: s.pose[2],
            },
            velocity: Vec2 {
                x: s.velocity[0],
                y: s.velocity[1],
            },
            applied_faults: serde_json::to_string(&s.applied_faults).unwrap(),
        }
    }
}
impl_from_plain_with_from_ref!(GNSSObservation, GNSSObservationWrapper);

impl From<&GNSSObservationWrapper> for GNSSObservation {
    /// Convert this wrapper to the Rust [`GNSSObservation`] type.
    fn from(value: &GNSSObservationWrapper) -> GNSSObservation {
        GNSSObservation {
            pose: Vector3::from_vec(vec![value.pose.x, value.pose.y, value.pose.z]),
            velocity: Vector2::from_vec(vec![value.velocity.x, value.velocity.y]),
            applied_faults: serde_json::from_str(&value.applied_faults).unwrap(),
        }
    }
}
impl_from_plain_with_from_ref!(GNSSObservationWrapper, GNSSObservation);

impl Default for GNSSObservationWrapper {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
#[pyclass(get_all)]
#[pyo3(name = "DisplacementObservation")]
/// Python wrapper around incremental displacement measurement.
pub struct DisplacementObservationWrapper {
    /// Translation increment on the xy plane.
    pub translation: Vec2,
    /// Rotation increment (radians).
    pub rotation: f32,
    /// Applied faults in JSON format
    pub applied_faults: String,
}

#[pymethods]
impl DisplacementObservationWrapper {
    /// Create a default displacement observation.
    #[new]
    pub fn new() -> Self {
        Self {
            translation: Vec2 { x: 0., y: 0. },
            rotation: 0.,
            applied_faults: "[]".to_string(),
        }
    }
}

impl From<&DisplacementObservation> for DisplacementObservationWrapper {
    /// Convert from the Rust [`DisplacementObservation`] type.
    fn from(s: &DisplacementObservation) -> Self {
        Self {
            translation: Vec2 {
                x: s.translation[0],
                y: s.translation[1],
            },
            rotation: s.rotation,
            applied_faults: serde_json::to_string(&s.applied_faults).unwrap(),
        }
    }
}
impl_from_plain_with_from_ref!(DisplacementObservation, DisplacementObservationWrapper);

impl From<&DisplacementObservationWrapper> for DisplacementObservation {
    /// Convert this wrapper to the Rust [`DisplacementObservation`] type.
    fn from(value: &DisplacementObservationWrapper) -> DisplacementObservation {
        DisplacementObservation {
            translation: Vector2::from_vec(vec![value.translation.x, value.translation.y]),
            rotation: value.rotation,
            applied_faults: serde_json::from_str(&value.applied_faults).unwrap(),
        }
    }
}
impl_from_plain_with_from_ref!(DisplacementObservationWrapper, DisplacementObservation);

impl Default for DisplacementObservationWrapper {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
#[pyclass(get_all, set_all)]
#[pyo3(name = "OrientedRobotObservation")]
/// Python wrapper around oriented robot observation.
pub struct OrientedRobotObservationWrapper {
    /// Name of the Robot
    pub name: String,
    /// Labels of the Robot
    pub labels: Vec<String>,
    /// Pose of the Robot
    pub pose: Pose,
    /// Applied faults in JSON format
    pub applied_faults: String,
}

#[pymethods]
impl OrientedRobotObservationWrapper {
    /// Create a default-oriented robot observation.
    #[new]
    pub fn new() -> Self {
        Self {
            name: "NoName".to_string(),
            labels: Vec::new(),
            pose: Pose {
                x: 0.,
                y: 0.,
                theta: 0.,
            },
            applied_faults: "[]".to_string(),
        }
    }
}

impl From<&OrientedRobotObservation> for OrientedRobotObservationWrapper {
    /// Convert from the Rust [`OrientedRobotObservation`] type.
    fn from(s: &OrientedRobotObservation) -> Self {
        Self {
            name: s.name.clone(),
            labels: s.labels.clone(),
            pose: Pose {
                x: s.pose[0],
                y: s.pose[1],
                theta: s.pose[2],
            },
            applied_faults: serde_json::to_string(&s.applied_faults).unwrap(),
        }
    }
}
impl_from_plain_with_from_ref!(OrientedRobotObservation, OrientedRobotObservationWrapper);

impl From<&OrientedRobotObservationWrapper> for OrientedRobotObservation {
    /// Convert this wrapper to the Rust [`OrientedRobotObservation`] type.
    fn from(value: &OrientedRobotObservationWrapper) -> OrientedRobotObservation {
        OrientedRobotObservation {
            name: value.name.clone(),
            labels: value.labels.clone(),
            pose: SVector::from_vec(vec![value.pose.x, value.pose.y, value.pose.theta]),
            applied_faults: serde_json::from_str(&value.applied_faults).unwrap(),
        }
    }
}
impl_from_plain_with_from_ref!(OrientedRobotObservationWrapper, OrientedRobotObservation);

impl Default for OrientedRobotObservationWrapper {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, EnumToString, Debug)]
#[pyclass(get_all, set_all)]
#[pyo3(name = "SensorObservation")]
/// Tagged union of supported sensor observations exposed to Python.
pub enum SensorObservationWrapper {
    /// Observation from an oriented-landmark sensor.
    OrientedLandmark(OrientedLandmarkObservationWrapper),
    /// Observation from a speed sensor.
    Speed(SpeedObservationWrapper),
    /// Observation from a GNSS-like sensor.
    GNSS(GNSSObservationWrapper),
    /// Observation from an oriented-robot sensor.
    OrientedRobot(OrientedRobotObservationWrapper),
    /// Observation from a displacement sensor.
    Displacement(DisplacementObservationWrapper),
}

#[pymethods]
impl SensorObservationWrapper {
    #[new]
    /// Create a default observation (GNSS for placeholder purposes).
    pub fn new() -> Self {
        SensorObservationWrapper::GNSS(GNSSObservationWrapper::new())
    }

    /// Try to convert the observation to an [`OrientedLandmarkObservationWrapper`].
    pub fn as_oriented_landmark(&self) -> PyResult<OrientedLandmarkObservationWrapper> {
        if let Self::OrientedLandmark(o) = self {
            Ok(o.clone())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Impossible to convert this observation to an OrientedLandmarkObservation",
            ))
        }
    }

    /// Try to convert the observation to a [`SpeedObservationWrapper`].
    pub fn as_speed(&self) -> PyResult<SpeedObservationWrapper> {
        if let Self::Speed(o) = self {
            Ok(o.clone())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Impossible to convert this observation to a SpeedObservation",
            ))
        }
    }

    /// Try to convert the observation to a [`GNSSObservationWrapper`].
    pub fn as_gnss(&self) -> PyResult<GNSSObservationWrapper> {
        if let Self::GNSS(o) = self {
            Ok(o.clone())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Impossible to convert this observation to an GNSSObservation",
            ))
        }
    }

    /// Try to convert the observation to an [`OrientedRobotObservationWrapper`].
    pub fn as_oriented_robot(&self) -> PyResult<OrientedRobotObservationWrapper> {
        if let Self::OrientedRobot(o) = self {
            Ok(o.clone())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Impossible to convert this observation to an OrientedRobotObservation",
            ))
        }
    }

    /// Try to convert the observation to a [`DisplacementObservationWrapper`].
    pub fn as_displacement(&self) -> PyResult<DisplacementObservationWrapper> {
        if let Self::Displacement(o) = self {
            Ok(o.clone())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Impossible to convert this observation to a DisplacementObservation",
            ))
        }
    }

    #[getter]
    /// Return the variant name as a string.
    pub fn kind(&self) -> String {
        self.to_string()
    }
}

impl From<&SensorObservation> for SensorObservationWrapper {
    /// Convert from the Rust [`SensorObservation`] type.
    fn from(s: &SensorObservation) -> Self {
        match s {
            SensorObservation::GNSS(o) => {
                SensorObservationWrapper::GNSS(GNSSObservationWrapper::from(o))
            }
            SensorObservation::Speed(o) => {
                SensorObservationWrapper::Speed(SpeedObservationWrapper::from(o))
            }
            SensorObservation::OrientedLandmark(o) => SensorObservationWrapper::OrientedLandmark(
                OrientedLandmarkObservationWrapper::from(o),
            ),
            SensorObservation::OrientedRobot(o) => {
                SensorObservationWrapper::OrientedRobot(OrientedRobotObservationWrapper::from(o))
            }
            SensorObservation::Displacement(o) => {
                SensorObservationWrapper::Displacement(DisplacementObservationWrapper::from(o))
            }
            SensorObservation::Scan(_) => {
                panic!("ScanObservation cannot be converted to SensorObservationWrapper yet");
            }
            SensorObservation::Plugin(_) => {
                panic!("PluginObservation cannot be converted to SensorObservationWrapper yet");
            }
        }
    }
}
impl_from_plain_with_from_ref!(SensorObservation, SensorObservationWrapper);

impl From<&SensorObservationWrapper> for SensorObservation {
    /// Convert this wrapper to the Rust [`SensorObservation`] type.
    fn from(value: &SensorObservationWrapper) -> SensorObservation {
        match value {
            SensorObservationWrapper::GNSS(o) => SensorObservation::GNSS(GNSSObservation::from(o)),
            SensorObservationWrapper::Speed(o) => {
                SensorObservation::Speed(SpeedObservation::from(o))
            }
            SensorObservationWrapper::OrientedLandmark(o) => {
                SensorObservation::OrientedLandmark(OrientedLandmarkObservation::from(o))
            }
            SensorObservationWrapper::OrientedRobot(o) => {
                SensorObservation::OrientedRobot(OrientedRobotObservation::from(o))
            }
            SensorObservationWrapper::Displacement(o) => {
                SensorObservation::Displacement(DisplacementObservation::from(o))
            }
        }
    }
}
impl_from_plain_with_from_ref!(SensorObservationWrapper, SensorObservation);

impl Default for SensorObservationWrapper {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
#[pyclass(get_all, set_all)]
#[pyo3(name = "Observation")]
/// Generic observation wrapper carrying metadata and observation from one sensor.
pub struct ObservationWrapper {
    /// Sensor identifier.
    pub sensor_name: String,
    /// Name of the node that observed.
    pub observer: String,
    /// Observation timestamp.
    pub time: f32,
    /// Concrete sensor observation payload.
    pub sensor_observation: SensorObservationWrapper,
    /// Frame of the sensor
    pub frame: Pose,
}

#[pymethods]
impl ObservationWrapper {
    /// Create a default observation with placeholder metadata.
    #[new]
    pub fn new() -> Self {
        Self {
            sensor_name: "some_sensor".to_string(),
            observer: "someone".to_string(),
            time: 0.,
            sensor_observation: SensorObservationWrapper::new(),
            frame: Pose {
                x: 0.,
                y: 0.,
                theta: 0.,
            },
        }
    }
}

impl From<&Observation> for ObservationWrapper {
    /// Convert from the Rust [`Observation`] type.
    fn from(s: &Observation) -> Self {
        Self {
            sensor_name: s.sensor_name.clone(),
            observer: s.observer.clone(),
            time: s.time,
            sensor_observation: SensorObservationWrapper::from(&s.sensor_observation),
            frame: Pose {
                x: s.frame.transform().translation.x,
                y: s.frame.transform().translation.y,
                theta: s.frame.transform().rotation.angle(),
            },
        }
    }
}
impl_from_plain_with_from_ref!(Observation, ObservationWrapper);

impl From<&ObservationWrapper> for Observation {
    /// Convert this wrapper to the Rust [`Observation`] type.
    fn from(value: &ObservationWrapper) -> Observation {
        Observation {
            sensor_name: value.sensor_name.clone(),
            observer: value.observer.clone(),
            time: value.time,
            sensor_observation: SensorObservation::from(&value.sensor_observation),
            frame: Frame::new(value.frame.x, value.frame.y, value.frame.theta),
        }
    }
}
impl_from_plain_with_from_ref!(ObservationWrapper, Observation);

impl Default for ObservationWrapper {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrapper around the [`Command`] to be used in Python.
/// It can be either a [`UnicycleCommandWrapper`] or a [`HolonomicCommandWrapper`].
#[derive(Clone, EnumToString, Debug)]
#[pyclass(get_all, set_all)]
#[pyo3(name = "Command")]
pub enum CommandWrapper {
    /// Command for unicycle robot models, containing the left and right wheel speeds.
    Unicycle(UnicycleCommandWrapper),
    /// Command for holonomic robot models, containing the longitudinal, lateral and angular velocities.
    Holonomic(HolonomicCommandWrapper),
}

#[pymethods]
impl CommandWrapper {
    /// Creates a new [`CommandWrapper`] with a default [`UnicycleCommandWrapper`]. It can be changed later using the setter or the static methods.
    #[new]
    pub fn new() -> CommandWrapper {
        Self::Unicycle(UnicycleCommandWrapper::new())
    }

    /// Tries to convert the command to a [`UnicycleCommandWrapper`]. It returns an error if the command is not a [`UnicycleCommandWrapper`].
    pub fn as_unicycle_command(&self) -> PyResult<UnicycleCommandWrapper> {
        if let Self::Unicycle(o) = self {
            Ok(o.clone())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Impossible to convert this command to a UnicycleCommand",
            ))
        }
    }

    /// Tries to convert the command to a [`HolonomicCommandWrapper`]. It returns an error if the command is not a [`HolonomicCommandWrapper`].
    pub fn as_holonomic_command(&self) -> PyResult<HolonomicCommandWrapper> {
        if let Self::Holonomic(o) = self {
            Ok(o.clone())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Impossible to convert this command to a HolonomicCommand",
            ))
        }
    }

    /// Returns the kind of the command as a string. It can be either "Unicycle" or "Holonomic".
    #[getter]
    pub fn kind(&self) -> String {
        self.to_string()
    }

    /// Creates a new [`CommandWrapper`] from a [`UnicycleCommandWrapper`].
    #[staticmethod]
    pub fn from_unicycle_command(cmd: UnicycleCommandWrapper) -> CommandWrapper {
        Self::Unicycle(cmd)
    }

    /// Creates a new [`CommandWrapper`] from a [`HolonomicCommandWrapper`].
    #[staticmethod]
    pub fn from_holonomic_command(cmd: HolonomicCommandWrapper) -> CommandWrapper {
        Self::Holonomic(cmd)
    }
}

impl Default for CommandWrapper {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&Command> for CommandWrapper {
    /// Creates a [`CommandWrapper`] from its Rust counterpart [`Command`]
    fn from(s: &Command) -> Self {
        match s {
            Command::Unicycle(cmd) => CommandWrapper::Unicycle(UnicycleCommandWrapper::from(cmd)),
            Command::Holonomic(cmd) => {
                CommandWrapper::Holonomic(HolonomicCommandWrapper::from(cmd))
            }
        }
    }
}
impl_from_plain_with_from_ref!(Command, CommandWrapper);

impl From<&CommandWrapper> for Command {
    /// Converts the Python wrapper back to its Rust counterpart [`Command`]
    fn from(value: &CommandWrapper) -> Command {
        match value {
            CommandWrapper::Unicycle(cmd) => Command::Unicycle(UnicycleCommand::from(cmd)),
            CommandWrapper::Holonomic(cmd) => Command::Holonomic(HolonomicCommand::from(cmd)),
        }
    }
}
impl_from_plain_with_from_ref!(CommandWrapper, Command);

/// Wrapper around the [`UnicycleCommand`] to be used in Python.
#[derive(Clone, Debug)]
#[pyclass(get_all, set_all)]
#[pyo3(name = "UnicycleCommand")]
pub struct UnicycleCommandWrapper {
    /// Left wheel speed.
    pub left_wheel_speed: f32,
    /// Right wheel speed.
    pub right_wheel_speed: f32,
}

#[pymethods]
impl UnicycleCommandWrapper {
    /// Creates a new [`UnicycleCommandWrapper`] with all wheel speeds set to 0.
    #[new]
    pub fn new() -> UnicycleCommandWrapper {
        Self {
            left_wheel_speed: 0.,
            right_wheel_speed: 0.,
        }
    }
}

impl Default for UnicycleCommandWrapper {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&UnicycleCommand> for UnicycleCommandWrapper {
    /// Creates an [`UnicycleCommandWrapper`] from its Rust counterpart [`UnicycleCommand`]
    fn from(s: &UnicycleCommand) -> Self {
        Self {
            left_wheel_speed: s.left_wheel_speed,
            right_wheel_speed: s.right_wheel_speed,
        }
    }
}
impl_from_plain_with_from_ref!(UnicycleCommand, UnicycleCommandWrapper);

impl From<&UnicycleCommandWrapper> for UnicycleCommand {
    /// Converts the Python wrapper back to its Rust counterpart [`UnicycleCommand`]
    fn from(value: &UnicycleCommandWrapper) -> UnicycleCommand {
        UnicycleCommand {
            left_wheel_speed: value.left_wheel_speed,
            right_wheel_speed: value.right_wheel_speed,
        }
    }
}
impl_from_plain_with_from_ref!(UnicycleCommandWrapper, UnicycleCommand);

/// Wrapper around the [`HolonomicCommand`] to be used in Python.
#[derive(Clone, Debug)]
#[pyclass(get_all, set_all)]
#[pyo3(name = "HolonomicCommand")]
pub struct HolonomicCommandWrapper {
    /// Longitudinal velocity (following x axis of the robot).
    pub longitudinal_velocity: f32,
    /// Lateral velocity (following y axis of the robot).
    pub lateral_velocity: f32,
    /// Angular velocity (radians per second).
    pub angular_velocity: f32,
}

#[pymethods]
impl HolonomicCommandWrapper {
    /// Creates a new [`HolonomicCommandWrapper`] with all velocities set to 0.
    #[new]
    pub fn new() -> HolonomicCommandWrapper {
        Self {
            longitudinal_velocity: 0.,
            lateral_velocity: 0.,
            angular_velocity: 0.,
        }
    }
}

impl Default for HolonomicCommandWrapper {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&HolonomicCommand> for HolonomicCommandWrapper {
    fn from(s: &HolonomicCommand) -> Self {
        Self {
            longitudinal_velocity: s.longitudinal_velocity,
            lateral_velocity: s.lateral_velocity,
            angular_velocity: s.angular_velocity,
        }
    }
}
impl_from_plain_with_from_ref!(HolonomicCommand, HolonomicCommandWrapper);

impl From<&HolonomicCommandWrapper> for HolonomicCommand {
    fn from(value: &HolonomicCommandWrapper) -> Self {
        Self {
            longitudinal_velocity: value.longitudinal_velocity,
            lateral_velocity: value.lateral_velocity,
            angular_velocity: value.angular_velocity,
        }
    }
}
impl_from_plain_with_from_ref!(HolonomicCommandWrapper, HolonomicCommand);

/// Wrapper for the [`Envelope`]
#[derive(Clone, Debug)]
#[pyclass(get_all, set_all)]
#[pyo3(name = "Envelope")]
pub struct EnvelopeWrapper {
    /// Expeditor
    pub msg_from: String,
    /// Message sent, see [`MessageTypeWrapper`] for supported messages
    pub message: MessageTypeWrapper,
    /// Time of the message
    pub timestamp: f32,
}

/// Wrapper around the [`Node`] to be used in Python.
/// It provides methods around the [`Network`].
#[derive(Debug, Clone)]
#[pyclass]
#[pyo3(name = "Node")]
pub struct NodeWrapper {
    name: String,
    network: Option<Weak<RwLock<Network>>>,
    context: Context,
}

#[pymethods]
impl NodeWrapper {
    /// Get the unique name of the node.
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Send a message to the given channel using the node [`Network`]. It returns an error if the node is not connected to any network.
    ///
    /// Should only be used for one-shot messages as it creates a new client every times. For more persistent communication, it is better to use a [`MultiClientWrapper`] obtained with the [`subscribe`](Self::subscribe) method.
    ///
    /// # Arguments
    /// * `to` - Channel key to send the message to. It can be a relative key (e.g. "channel1") or an absolute key (e.g. "/my_channels/channel1").
    /// * `message` - Message to send. See [`MessageTypeWrapper`] for the supported message types in Python.
    /// * `time` - Time at which the message is sent. It should be the current time of the simulator or in the future.
    /// * `flags` - Message flags. See [`MessageFlag`](::simusa::networking::network::MessageFlag) for the supported flags.
    #[pyo3(signature = (to, message, time, flags=Vec::new()))]
    #[warn(clippy::useless_conversion)]
    pub fn send_message(
        &self,
        to: String,
        message: MessageTypeWrapper,
        time: f32,
        flags: Vec<MessageFlagWrapper>,
    ) -> PyResult<()> {
        if let Some(network) = self.network.as_ref().and_then(|n| n.upgrade()) {
            let msg = message.to_rust(&self.context);
            let key = PathKey::from_str(to.as_str()).unwrap();
            let msg = Envelope {
                from: self.name.clone(),
                message: msg,
                timestamp: time,
                message_flags: flags.iter().map(|m| m.to_rust()).collect(),
            };
            network
                .write()
                .unwrap()
                .send_to(key, msg, time, &self.context);
            Ok(())
        } else {
            Err(PyErr::new::<PyTypeError, _>("No network on this node"))
        }
    }

    /// Subscribe to a channel using the node [`Network`] and get a [`MultiClientWrapper`] to send and receive messages on this channel.
    /// It returns an error if the node is not connected to any network.
    pub fn subscribe(&self, channels: Vec<String>) -> PyResult<MultiClientWrapper> {
        if let Some(network) = &self.network.as_ref().and_then(|n| n.upgrade()) {
            let keys = channels
                .iter()
                .map(|t| PathKey::from_str(t.as_str()).unwrap())
                .collect::<Vec<PathKey>>();
            let client = network
                .write()
                .unwrap()
                .subscribe_to(&keys, None, &self.context);
            Ok(MultiClientWrapper::from_rust(client, self.context.clone()))
        } else {
            Err(PyErr::new::<PyTypeError, _>("No network on this node"))
        }
    }

    /// Create a new channel in the network using [`Network::make_channel`]. It returns an error if the node is not connected to any network.
    pub fn make_channel(&self, channel_name: String) -> PyResult<()> {
        if let Some(network) = &self.network.as_ref().and_then(|n| n.upgrade()) {
            let key = PathKey::from_str(channel_name.as_str()).unwrap();
            network.write().unwrap().make_channel(key, &self.context);
            Ok(())
        } else {
            Err(PyErr::new::<PyTypeError, _>("No network on this node"))
        }
    }

    /// Log an info message using the simulator's logging system.
    pub fn log_info(&self, message: String) {
        info!(self.context, "{}", message);
    }

    /// Log a warning message using the simulator's logging system.
    pub fn log_warning(&self, message: String) {
        warning!(self.context, "{}", message);
    }

    /// Log an error message using the simulator's logging system.
    pub fn log_error(&self, message: String) {
        error!(self.context, "{}", message);
    }

    /// Log a debug message using the simulator's logging system.
    pub fn log_debug(&self, message: String) {
        debug!(self.context, "{}", message);
    }
}

impl NodeWrapper {
    /// Create a new NodeWrapper from a Node.
    pub fn from_rust(n: &Node, context: Context) -> Self {
        Self {
            name: n.name(),
            network: n.network().as_ref().map(Arc::downgrade),
            context,
        }
    }
}

/// Wrapper around the [`SimusaBrokerMultiClient`] to be used in Python.
/// It provides methods to subscribe to channels, send messages and receive messages.
#[derive(Debug)]
#[pyclass]
#[pyo3(name = "Client")]
pub struct MultiClientWrapper {
    client: SimusaBrokerMultiClient,
    context: Context,
}

#[pymethods]
impl MultiClientWrapper {
    /// Subscribe to a channel. The client will receive messages sent to this channel.
    ///
    /// If the channel key is relative (does not start with a "/"), the subscription will be made to the channel using
    /// the node id as prefix. For example, if the node id is "node1" and the channel key is "channel1", the subscription will be made to "/simusa/nodes/node1/channel1".
    ///
    /// # Arguments
    /// * `key` - Channel key to subscribe to. It can be a relative key (e.g. "channel1") or an absolute key (e.g. "/my_channels/channel1").
    pub fn subscribe(&mut self, key: String) {
        let key = PathKey::from_str(key.as_str()).unwrap();
        self.client.subscribe(&key);
    }

    /// Similar to [`subscribe`](Self::subscribe) but it overrides the network delays.
    /// To be used for meta messages, or virtual messages that would not exists in a real world
    pub fn subscribe_instantaneous(&mut self, key: String) {
        let key = PathKey::from_str(key.as_str()).unwrap();
        self.client.subscribe_instantaneous(&key);
    }

    /// Send a message to the given channel. The message will be received by all the clients subscribed to this channel which are in range.
    ///
    /// # Arguments
    /// * `to` - Channel key to send the message to. It can be a relative key (e.g. "channel1") or an absolute key (e.g. "/my_channels/channel1").
    /// * `message` - Message to send. See [`MessageTypeWrapper`] for the supported message types in Python.
    /// * `time` - Time at which the message is sent. It should be the current time of the simulator or in the future.
    /// * `flags` - Message flags. See [`MessageFlag`](::simusa::networking::network::MessageFlag) for the supported flags.
    pub fn send(
        &self,
        to: String,
        message: MessageTypeWrapper,
        time: f32,
        flags: Vec<MessageFlagWrapper>,
    ) -> PyResult<()> {
        let msg = message.to_rust(&self.context);
        let key = PathKey::from_str(to.as_str()).unwrap();
        let msg = Envelope {
            from: self.client.node_id().to_string(),
            message: msg,
            timestamp: time,
            message_flags: flags.iter().map(|m| m.to_rust()).collect(),
        };
        self.client.send(&key, msg, time);
        Ok(())
    }

    /// Try to receive a message from the subscribed channels. It returns the first message received or None if no message is received within the given time.
    ///
    /// It can return messages older than the given `time`.
    pub fn try_receive(&self, time: f32) -> Option<(String, EnvelopeWrapper)> {
        if let Some((path, envelope)) = self.client.try_receive(time) {
            Some((
                path.to_string(),
                EnvelopeWrapper {
                    msg_from: envelope.from,
                    message: MessageTypeWrapper::from_rust(&envelope.message),
                    timestamp: envelope.timestamp,
                },
            ))
        } else {
            None
        }
    }

    /// Get the time of the next message to be received. It returns None if no message is expected to be received in the future.
    pub fn next_message_time(&self) -> Option<f32> {
        self.client.next_message_time()
    }

    /// Get the list of channels the client is subscribed to.
    pub fn subscribed_keys(&self) -> Vec<String> {
        self.client
            .subscribed_keys()
            .iter()
            .map(|k| k.to_string())
            .collect()
    }

    /// Get the node id of the client. It is the same as the name of the node the client is attached to.
    pub fn node_id(&self) -> String {
        self.client.node_id().to_string()
    }
}

impl MultiClientWrapper {
    /// Create a new MultiClientWrapper from a SimusaBrokerMultiClient.
    pub fn from_rust(client: SimusaBrokerMultiClient, context: Context) -> Self {
        Self { client, context }
    }
}

/// Trait to be implemented by the Python API provided by the user. It provides methods to get the state estimator, controller, navigator and physics to be used by the simulator.
/// The user can implement this trait in Python and provide it to the simulator to use custom implementations of these components.
///
/// # Example
/// ```python
/// import simusa
/// import json
///
/// class Controller(simusa.Controller):
///     # implementation
///
/// class SimulatorAPI(simusa.PluginAPI):
///     def get_controller(self, config: dict, global_config: dict, initial_time: float):
///         config = json.loads(config)
///         print(f"Config received by python: {type(config)} {config}")
///         return Controller(config, initial_time)
///
/// def main():
///     simulator_api = SimulatorAPI()
///
///     simulator = simusa.Simulator.from_config(
///         "config/config_controller.yaml", simulator_api
///     )
///     simulator.run()
/// ```
#[pyclass(subclass)]
#[pyo3(name = "PluginAPI")]
pub struct PluginAPIWrapper {}

#[pymethods]
impl PluginAPIWrapper {
    /// Constructor for the PluginAPI. It does not take any argument and returns an instance of the PluginAPI.
    #[new]
    pub fn new() -> Self {
        Self {}
    }
    /// Return the [`StateEstimator`](::simusa::state_estimators::StateEstimator)
    ///
    /// # Arguments
    /// * `config` - Config for the external state estimator. The configuration
    /// is given using [`::simusa::utils::GeneralValue`]. It should be converted by the
    /// external plugin to the specific configuration.
    /// * `global_config` - Full configuration of the simulator.
    ///
    /// # Return
    ///
    /// Returns the [`StateEstimator`](::simusa::state_estimators::StateEstimator) to use.
    pub fn get_state_estimator(
        &self,
        _config: Py<PyAny>,
        _global_config: Py<PyAny>,
        _initial_time: f32,
    ) -> StateEstimatorWrapper {
        panic!("The given PluginAPI does not provide a state estimator");
    }

    /// Return the [`Controller`](::simusa::controllers::Controller).
    ///
    /// # Arguments
    /// * `config` - Config for the external controller. The configuration
    /// is given using [`::simusa::utils::GeneralValue`]. It should be converted by the
    /// external plugin to the specific configuration.
    /// * `global_config` - Full configuration of the simulator.
    ///
    /// # Return
    ///
    /// Returns the [`Controller`](::simusa::controllers::Controller) to use.
    pub fn get_controller(
        &self,
        _config: Py<PyAny>,
        _global_config: Py<PyAny>,
        _initial_time: f32,
    ) -> ControllerWrapper {
        panic!("The given PluginAPI does not provide a controller");
    }

    /// Return the [`Navigator`](::simusa::navigators::Navigator).
    ///
    /// # Arguments
    /// * `config` - Config for the external navigator. The configuration
    /// is given using [`::simusa::utils::GeneralValue`]. It should be converted by the
    /// external plugin to the specific configuration.
    /// * `global_config` - Full configuration of the simulator.
    ///
    /// # Return
    ///
    /// Returns the [`Navigator`](::simusa::navigators::Navigator) to use.
    pub fn get_navigator(
        &self,
        _config: Py<PyAny>,
        _global_config: Py<PyAny>,
        _initial_time: f32,
    ) -> NavigatorWrapper {
        panic!("The given PluginAPI does not provide a navigator");
    }

    /// Return the [`Physics`](::simusa::physics::Physics).
    ///
    /// # Arguments
    /// * `config` - Config for the external physics. The configuration
    /// is given using [`::simusa::utils::GeneralValue`]. It should be converted by the
    /// external plugin to the specific configuration.
    /// * `global_config` - Full configuration of the simulator.
    ///
    /// # Return
    ///
    /// Returns the [`Physics`](::simusa::physics::Physics) to use.
    pub fn get_physics(
        &self,
        _config: Py<PyAny>,
        _global_config: Py<PyAny>,
        _initial_time: f32,
    ) -> PhysicsWrapper {
        panic!("The given PluginAPI does not provide physics");
    }
}

impl Default for PluginAPIWrapper {
    fn default() -> Self {
        Self::new()
    }
}

/// Simulator wrapper to be used in Python. It provides a method to create a simulator from a configuration file and a method to run the simulator.
#[pyclass]
#[pyo3(name = "Simulator")]
pub struct SimulatorWrapper {
    simulator: AsyncSimulator,
    python_api: Option<Arc<dyn PluginAPI>>,
}

#[pymethods]
impl SimulatorWrapper {
    /// Constructor for the simulator.
    ///
    /// # Arguments
    /// * `config_path` - Path to the configuration file. The configuration file should be in YAML format and follow the structure defined in the documentation. See the [`SimulatorConfig`](::simusa::simulator::SimulatorConfig) for more details on the configuration structure.
    /// * `plugin_api` - Optional Python object that implements the [`PluginAPI`] interface. This object will be used to provide custom implementations of the state estimator, controller, navigator and physics if required by the configuration.
    #[staticmethod]
    #[pyo3(signature = (config_path, plugin_api=None))]
    pub fn from_config(
        config_path: String,
        plugin_api: Option<Py<PyAny>>,
    ) -> PyResult<SimulatorWrapper> {
        let python_api = plugin_api.map(|api| Arc::new(PythonAPI::new(api)) as Arc<dyn PluginAPI>);

        let simulator =
            AsyncSimulator::from_config_path(&config_path, &python_api).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to create simulator from config: {}",
                    e.detailed_error()
                ))
            })?;
        Ok(SimulatorWrapper {
            simulator,
            python_api,
        })
    }

    /// Run the simulator. This function will block until the simulation is finished.
    /// It will call the given state estimator, controller, navigator and physics at each simulation step.
    pub fn run(&mut self) {
        self.simulator.run(&self.python_api, None, false);
        self.simulator.compute_results();
        self.simulator.stop();
    }
}
