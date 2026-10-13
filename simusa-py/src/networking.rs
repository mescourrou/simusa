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
#[pyo3(name = "MessageType")]
/// Python wrapper for the Rust enum [`MessageType`], which represents different types of messages that can be sent through the network.
pub enum MessageTypeWrapper {
    /// Arbitrary UTF-8 textual payload.
    String(String),
    /// Navigation payload used by [`GoToMessage`](::simusa::navigators::go_to::GoToMessage).
    GoTo(GoToMessageWrapper),
    /// Sensor event payload used by [`SensorTriggerMessage`](::simusa::sensors::sensor_manager::SensorTriggerMessage).
    SensorTrigger(SensorTriggerMessageWrapper),
    /// A list of observations, each wrapped in an [`ObservationWrapper`].
    Observations(Vec<ObservationWrapper>),
    /// Setting the frame of a sensor, used by [`SensorManager`](::simusa::sensors::sensor_manager::SensorManager).
    SensorFrame(SensorFrameMessageWrapper),
    /// An event record serialized as a JSON string.
    Event(String),
    /// Custom binary payload.
    Custom(Vec<u8>),
}
#[pymethods]
impl MessageTypeWrapper {
    /// Creates a [`MessageType::GoTo`] from a [`GoToMessage`](::simusa::navigators::go_to::GoToMessage).
    #[staticmethod]
    pub fn from_goto(message: GoToMessageWrapper) -> Self {
        MessageTypeWrapper::GoTo(message)
    }

    /// Creates a [`MessageType::SensorTrigger`] from a [`SensorTriggerMessage`](::simusa::sensors::sensor_manager::SensorTriggerMessage).
    #[staticmethod]
    pub fn from_sensor_trigger(message: SensorTriggerMessageWrapper) -> Self {
        MessageTypeWrapper::SensorTrigger(message)
    }

    /// Creates a [`MessageType::SensorFrame`] from a [`SensorFrameMessage`](::simusa::sensors::sensor_manager::SensorFrameMessage).
    #[staticmethod]
    pub fn from_sensor_frame(message: SensorFrameMessageWrapper) -> Self {
        MessageTypeWrapper::SensorFrame(message)
    }

    /// Creates a [`MessageType::String`] from a string.
    #[staticmethod]
    pub fn from_string(message: String) -> Self {
        MessageTypeWrapper::String(message)
    }

    /// Creates a [`MessageType::Observations`] from a list of [`ObservationWrapper`].
    #[staticmethod]
    pub fn from_observations(observations: Vec<ObservationWrapper>) -> Self {
        MessageTypeWrapper::Observations(observations)
    }

    /// Creates a [`MessageType::Event`] from a string.
    #[staticmethod]
    pub fn from_event(message: String) -> Self {
        MessageTypeWrapper::Event(message)
    }

    /// Creates a [`MessageType::Custom`] from binary data.
    #[staticmethod]
    pub fn from_custom(data: Vec<u8>) -> Self {
        MessageTypeWrapper::Custom(data)
    }

    /// Returns the contained [`GoToMessage`](::simusa::navigators::go_to::GoToMessage) when this value is [`MessageType::GoTo`].
    pub fn as_goto(&self) -> Option<GoToMessageWrapper> {
        match self {
            MessageTypeWrapper::GoTo(msg) => Some(msg.clone()),
            _ => None,
        }
    }

    /// Returns the contained [`SensorFrameMessage`](::simusa::sensors::sensor_manager::SensorFrameMessage) when this value is [`MessageType::SensorFrame`].
    pub fn as_sensor_frame(&self) -> Option<SensorFrameMessageWrapper> {
        match self {
            MessageTypeWrapper::SensorFrame(msg) => Some(msg.clone()),
            _ => None,
        }
    }

    /// Returns the contained [`SensorTriggerMessage`](::simusa::sensors::sensor_manager::SensorTriggerMessage) when this value is
    /// [`MessageType::SensorTrigger`].
    pub fn as_sensor_trigger(&self) -> Option<SensorTriggerMessageWrapper> {
        match self {
            MessageTypeWrapper::SensorTrigger(msg) => Some(msg.clone()),
            _ => None,
        }
    }

    /// Returns the contained string when this value is [`MessageType::String`].
    pub fn as_string(&self) -> Option<String> {
        match self {
            MessageTypeWrapper::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    /// Returns the contained [`ObservationWrapper`] when this value is [`MessageType::Observations`].
    pub fn as_observations(&self) -> Option<Vec<ObservationWrapper>> {
        match self {
            MessageTypeWrapper::Observations(obs) => Some(obs.clone()),
            _ => None,
        }
    }

    /// Returns the contained string when this value is [`MessageType::Event`].
    pub fn as_event(&self) -> Option<String> {
        match self {
            MessageTypeWrapper::Event(s) => Some(s.clone()),
            _ => None,
        }
    }

    /// Returns the contained binary data when this value is [`MessageType::Custom`].
    pub fn as_custom(&self) -> Option<Vec<u8>> {
        match self {
            MessageTypeWrapper::Custom(data) => Some(data.clone()),
            _ => None,
        }
    }

    /// Returns the variant discriminator as a string.
    #[getter]
    pub fn kind(&self) -> String {
        self.to_string()
    }
}

impl MessageTypeWrapper {
    /// Converts this Python wrapper back into the original Rust enum [`MessageType`].
    pub fn to_rust(&self, context: &Context) -> MessageType {
        match self {
            MessageTypeWrapper::String(s) => MessageType::String(s.clone()),
            MessageTypeWrapper::GoTo(msg) => MessageType::GoTo(msg.to_rust()),
            MessageTypeWrapper::SensorTrigger(msg) => MessageType::SensorTrigger(msg.to_rust()),
            MessageTypeWrapper::Observations(observations) => MessageType::Observations(
                observations
                    .iter()
                    .map(Observation::from)
                    .map(|o| o.record(context))
                    .collect(),
            ),
            MessageTypeWrapper::SensorFrame(msg) => MessageType::SensorFrame(msg.to_rust()),
            MessageTypeWrapper::Event(event) => MessageType::Event(
                serde_json::from_str(event).expect("Failed to parse EventRecord from JSON string"),
            ),
            MessageTypeWrapper::Custom(data) => MessageType::Custom(data.clone()),
        }
    }

    /// Creates a [`MessageTypeWrapper`] from a Rust enum [`MessageType`].
    pub fn from_rust(msg: &MessageType) -> Self {
        match msg {
            MessageType::String(s) => MessageTypeWrapper::String(s.clone()),
            MessageType::GoTo(msg) => MessageTypeWrapper::GoTo(GoToMessageWrapper::from_rust(msg)),
            MessageType::SensorTrigger(msg) => {
                MessageTypeWrapper::SensorTrigger(SensorTriggerMessageWrapper::from_rust(msg))
            }
            MessageType::SensorFrame(msg) => {
                MessageTypeWrapper::SensorFrame(SensorFrameMessageWrapper::from_rust(msg))
            }
            MessageType::Observations(observations) => MessageTypeWrapper::Observations(
                observations
                    .iter()
                    .map(|o| ObservationWrapper::from(&Observation::from_record(o)))
                    .collect(),
            ),
            MessageType::Event(event) => MessageTypeWrapper::Event(
                serde_json::to_string(event)
                    .expect("Failed to serialize EventRecord to JSON string"),
            ),
            MessageType::Custom(data) => MessageTypeWrapper::Custom(data.clone()),
        }
    }
}
