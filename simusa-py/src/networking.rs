//! Python bindings for the networking module of Simusa, allowing Python code to interact with Rust networking components.
//!
use pyo3::prelude::*;
use simusa_macros::EnumToString;

use simusa::{
    context::Context, networking::message_types::MessageType, recordable::Recordable,
    sensors::Observation,
};

use crate::{
    messages::{GoToMessageWrapper, SensorFrameMessageWrapper, SensorTriggerMessageWrapper},
    wrappers::ObservationWrapper,
};

#[derive(Clone, Debug, EnumToString)]
#[pyclass]
#[pyo3(name = "MessageTypes")]
/// Python wrapper for the Rust enum [`MessageTypes`], which represents different types of messages that can be sent through the network.
pub enum MessageTypesWrapper {
    /// Arbitrary UTF-8 textual payload.
    String(String),
    /// Navigation payload used by [`GoToMessage`].
    GoTo(GoToMessageWrapper),
    /// Sensor event payload used by [`SensorTriggerMessage`].
    SensorTrigger(SensorTriggerMessageWrapper),
    /// A list of observations, each wrapped in an [`ObservationWrapper`].
    Observations(Vec<ObservationWrapper>),
    /// Setting the frame of a sensor, used by [`SensorManager`](crate::sensors::sensor_manager::SensorManager).
    SensorFrame(SensorFrameMessageWrapper),
    /// An event record serialized as a JSON string.
    Event(String),
    /// Custom binary payload.
    Custom(Vec<u8>),
}
#[pymethods]
impl MessageTypesWrapper {
    /// Creates a [`MessageTypes::GoTo`] from a [`GoToMessage`].
    #[staticmethod]
    pub fn from_goto(message: GoToMessageWrapper) -> Self {
        MessageTypesWrapper::GoTo(message)
    }

    /// Creates a [`MessageTypes::SensorTrigger`] from a [`SensorTriggerMessage`].
    #[staticmethod]
    pub fn from_sensor_trigger(message: SensorTriggerMessageWrapper) -> Self {
        MessageTypesWrapper::SensorTrigger(message)
    }

    /// Creates a [`MessageTypes::SensorFrame`] from a [`SensorFrameMessage`].
    #[staticmethod]
    pub fn from_sensor_frame(message: SensorFrameMessageWrapper) -> Self {
        MessageTypesWrapper::SensorFrame(message)
    }

    /// Creates a [`MessageTypes::String`] from a string.
    #[staticmethod]
    pub fn from_string(message: String) -> Self {
        MessageTypesWrapper::String(message)
    }

    /// Creates a [`MessageTypes::Observations`] from a list of [`ObservationWrapper`].
    #[staticmethod]
    pub fn from_observations(observations: Vec<ObservationWrapper>) -> Self {
        MessageTypesWrapper::Observations(observations)
    }

    /// Creates a [`MessageTypes::Event`] from a string.
    #[staticmethod]
    pub fn from_event(message: String) -> Self {
        MessageTypesWrapper::Event(message)
    }

    /// Creates a [`MessageTypes::Custom`] from binary data.
    #[staticmethod]
    pub fn from_custom(data: Vec<u8>) -> Self {
        MessageTypesWrapper::Custom(data)
    }

    /// Returns the contained [`GoToMessage`] when this value is [`MessageTypes::GoTo`].
    pub fn as_goto(&self) -> Option<GoToMessageWrapper> {
        match self {
            MessageTypesWrapper::GoTo(msg) => Some(msg.clone()),
            _ => None,
        }
    }

    /// Returns the contained [`SensorFrameMessage`] when this value is [`MessageTypes::SensorFrame`].
    pub fn as_sensor_frame(&self) -> Option<SensorFrameMessageWrapper> {
        match self {
            MessageTypesWrapper::SensorFrame(msg) => Some(msg.clone()),
            _ => None,
        }
    }

    /// Returns the contained [`SensorTriggerMessage`] when this value is
    /// [`MessageTypes::SensorTrigger`].
    pub fn as_sensor_trigger(&self) -> Option<SensorTriggerMessageWrapper> {
        match self {
            MessageTypesWrapper::SensorTrigger(msg) => Some(msg.clone()),
            _ => None,
        }
    }

    /// Returns the contained string when this value is [`MessageTypes::String`].
    pub fn as_string(&self) -> Option<String> {
        match self {
            MessageTypesWrapper::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    /// Returns the contained [`ObservationWrapper`] when this value is [`MessageTypes::Observations`].
    pub fn as_observations(&self) -> Option<Vec<ObservationWrapper>> {
        match self {
            MessageTypesWrapper::Observations(obs) => Some(obs.clone()),
            _ => None,
        }
    }

    /// Returns the contained string when this value is [`MessageTypes::Event`].
    pub fn as_event(&self) -> Option<String> {
        match self {
            MessageTypesWrapper::Event(s) => Some(s.clone()),
            _ => None,
        }
    }

    /// Returns the contained binary data when this value is [`MessageTypes::Custom`].
    pub fn as_custom(&self) -> Option<Vec<u8>> {
        match self {
            MessageTypesWrapper::Custom(data) => Some(data.clone()),
            _ => None,
        }
    }

    /// Returns the variant discriminator as a string.
    #[getter]
    pub fn kind(&self) -> String {
        self.to_string()
    }
}

impl MessageTypesWrapper {
    /// Converts this Python wrapper back into the original Rust enum [`MessageTypes`].
    pub fn to_rust(&self, context: &Context) -> MessageType {
        match self {
            MessageTypesWrapper::String(s) => MessageType::String(s.clone()),
            MessageTypesWrapper::GoTo(msg) => MessageType::GoTo(msg.to_rust()),
            MessageTypesWrapper::SensorTrigger(msg) => MessageType::SensorTrigger(msg.to_rust()),
            MessageTypesWrapper::Observations(observations) => MessageType::Observations(
                observations
                    .iter()
                    .map(Observation::from)
                    .map(|o| o.record(context))
                    .collect(),
            ),
            MessageTypesWrapper::SensorFrame(msg) => MessageType::SensorFrame(msg.to_rust()),
            MessageTypesWrapper::Event(event) => MessageType::Event(
                serde_json::from_str(event).expect("Failed to parse EventRecord from JSON string"),
            ),
            MessageTypesWrapper::Custom(data) => MessageType::Custom(data.clone()),
        }
    }

    /// Creates a [`MessageTypesWrapper`] from a Rust enum [`MessageTypes`].
    pub fn from_rust(msg: &MessageType) -> Self {
        match msg {
            MessageType::String(s) => MessageTypesWrapper::String(s.clone()),
            MessageType::GoTo(msg) => MessageTypesWrapper::GoTo(GoToMessageWrapper::from_rust(msg)),
            MessageType::SensorTrigger(msg) => {
                MessageTypesWrapper::SensorTrigger(SensorTriggerMessageWrapper::from_rust(msg))
            }
            MessageType::SensorFrame(msg) => {
                MessageTypesWrapper::SensorFrame(SensorFrameMessageWrapper::from_rust(msg))
            }
            MessageType::Observations(observations) => MessageTypesWrapper::Observations(
                observations
                    .iter()
                    .map(|o| ObservationWrapper::from(&Observation::from_record(o)))
                    .collect(),
            ),
            MessageType::Event(event) => MessageTypesWrapper::Event(
                serde_json::to_string(event)
                    .expect("Failed to serialize EventRecord to JSON string"),
            ),
            MessageType::Custom(data) => MessageTypesWrapper::Custom(data.clone()),
        }
    }
}
