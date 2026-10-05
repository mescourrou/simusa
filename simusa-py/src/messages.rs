use simusa::{navigators::go_to::GoToMessage, networking::network::MessageFlag, sensors::sensor_manager::{SensorFrameMessage, SensorTriggerMessage}};
use simusa_macros::EnumToString;
use pyo3::prelude::*;

#[derive(Clone, Debug)]
#[pyclass(get_all, set_all)]
#[pyo3(name = "SensorTriggerMessage")]
pub struct SensorTriggerMessageWrapper {}

#[pymethods]
impl SensorTriggerMessageWrapper {
    /// Creates an empty trigger message.
    #[new]
    pub fn new() -> Self {
        Self {}
    }
}

impl SensorTriggerMessageWrapper {
    pub fn from_rust(msg: &SensorTriggerMessage) -> Self {
        Self {}
    }

    pub fn to_rust(&self) -> SensorTriggerMessage {
        SensorTriggerMessage {}
    }
}

#[derive(Clone, Debug)]

#[pyclass(get_all, set_all)]
#[pyo3(name = "SensorFrameMessage")]
pub struct SensorFrameMessageWrapper {
    /// New frame
    pub frame: [f32; 3],
    /// Name of the sensor (should already be created in the node)
    pub sensor_name: String,
}

#[pymethods]
impl SensorFrameMessageWrapper {
    /// Creates an empty frame message.
    #[new]
    pub fn new(frame: [f32; 3], sensor_name: String) -> Self {
        Self {
            frame,
            sensor_name,
        }
    }
}

impl SensorFrameMessageWrapper {
    pub fn from_rust(msg: &SensorFrameMessage) -> Self {
        Self {
            frame: msg.frame,
            sensor_name: msg.sensor_name.clone(),
        }
    }

    pub fn to_rust(&self) -> SensorFrameMessage {
        SensorFrameMessage {
            frame: self.frame,
            sensor_name: self.sensor_name.clone(),
        }
    }
}

#[derive(Clone, Debug)]
#[pyclass(get_all, set_all)]
#[pyo3(name = "GoToMessage")]
/// Runtime message used to update the target point of a [`GoTo`] navigator.
pub struct GoToMessageWrapper {
    /// Target point in world coordinates `[x, y]`.
    ///
    /// `None` clears the current target.
    pub target_point: Option<[f32; 2]>,
}

#[pymethods]
impl GoToMessageWrapper {
    /// Creates a new [`GoToMessageWrapper`] from an optional target point.
    #[new]
    #[pyo3(signature = (list=None))]
    pub fn new(list: Option<[f32; 2]>) -> Self {
        Self { target_point: list }
    }
}

impl GoToMessageWrapper {
    pub fn from_rust(msg: &GoToMessage) -> Self {
        Self {
            target_point: msg.target_point,
        }
    }

    pub fn to_rust(&self) -> GoToMessage {
        GoToMessage {
            target_point: self.target_point,
        }
    }
}

#[derive(Clone, Debug, EnumToString, PartialEq, Eq)]
#[pyclass(get_all, set_all, eq, eq_int)]
#[pyo3(name = "MessageFlag")]
pub enum MessageFlagWrapper {
    /// Bypass regular timing and treat the message as instantaneous.
    God,
    /// Ask the receiver to unsubscribe from the associated channel.
    Unsubscribe,
    /// Ask to terminate the receiving node.
    Kill,
}

#[pymethods]
impl MessageFlagWrapper {
    #[new]
    pub fn new(flag: &str) -> PyResult<Self> {
        match flag.to_lowercase().as_str() {
            "god" => Ok(Self::God),
            "unsubscribe" => Ok(Self::Unsubscribe),
            "kill" => Ok(Self::Kill),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid MessageFlag: {}. Valid values are: God, Unsubscribe, Kill",
                flag
            ))),
        }
    }

    #[getter]
    pub fn kind(&self) -> String {
        match self {
            Self::God => "God".to_string(),
            Self::Unsubscribe => "Unsubscribe".to_string(),
            Self::Kill => "Kill".to_string(),
        }
    }
}

impl MessageFlagWrapper {
    pub fn from_rust(flag: &MessageFlag) -> Self {
        match flag {
            MessageFlag::God => Self::God,
            MessageFlag::Unsubscribe => Self::Unsubscribe,
            MessageFlag::Kill => Self::Kill,
        }
    }

    pub fn to_rust(&self) -> MessageFlag {
        match self {
            Self::God => MessageFlag::God,
            Self::Unsubscribe => MessageFlag::Unsubscribe,
            Self::Kill => MessageFlag::Kill,
        }
    }
    
}