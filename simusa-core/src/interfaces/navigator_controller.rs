//! This module defines the interface between navigators and controllers in the Simusa framework.
//!
//! This interface is around the [`ControllerInputVariable`] enum, which defines the variables that
//! can be requested by the controller from the navigator.
//!
//! It provides the necessary structures, enums, and macros to facilitate communication and data
//! exchange between these two components.
#[cfg(feature = "gui")]
use crate::gui::UIComponent;
use crate::{
    context::Context,
    recordable::Recordable,
    state_estimators::{WorldState, WorldStateRecord},
    utils::GeneralValue,
};

use serde_derive::{Deserialize, Serialize};

/// Generate the controller input variables and macros to be used by specific navigators and controllers.
///
/// Syntax:
/// ```text
/// (
///     $camel_case_name: ident, // Name of the variable in CamelCase
///     $snake_case_name: ident, // Name of the variable in snake_case
///     $t: ty, // Type of the variable
///     $t_record: ty, // Type of the variable for the record (need to be serializable)
///     $t_python: ty, // Type of the variable for the python wrapper
///     $docstring: expr // Documentation string for the variable
/// );+
/// ```
///
/// This generates the following:
/// - An enum [`ControllerInputVariable`] with variants for each variable.
/// - A struct [`ControllerInputVariables`] with optional fields for each variable.
/// - A struct [`ControllerInputRecord`] with optional fields for each variable, used for recording types.
/// - A macro [`declare_controller_input!`] to declare a controller input struct with the specified variables,
///   implementing the [`ControllerInput`](crate::controllers::ControllerInput) trait and conversion from [`ControllerInputVariables`].
/// - A macro [`declare_easy_navigator_output!`] to declare a navigator output struct with the specified variables, implementing the
///   [`NavigatorOutput`](crate::navigators::NavigatorOutput) trait and conversion to [`ControllerInputVariables`]. All variables present
///   in the struct should be computed by the navigator.
/// - A macro [`declare_optimized_navigator_output!`] to declare a navigator output struct with the specified variables, implementing the
///   [`NavigatorOutput`](crate::navigators::NavigatorOutput) trait and conversion to [`ControllerInputVariables`]. Only the variables
///   requested by the controller will be computed by the navigator.
///
macro_rules! generate_controller_input {
    ($(
        $camel_case_name: ident,
        $snake_case_name: ident,
        $t: ty, // Type
        $t_record: ty, // Type for the record (need to be serializable)
        $t_python: ty, // Type for the python wrapper
        $docstring: expr // Documentation string for the variable
    );+ $(;)?) => {
/****************************************************************************************************
 * ControllerInputVariable affiliated enums and structs
 ***************************************************************************************************/


/// Enumerates the controller input variables that can be requested from the navigator
/// by the controller. Each variable corresponds to a specific field in [`ControllerInput`].
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub enum ControllerInputVariable {
    $(
        /// Variable corresponding to the `$snake_case_name` field in [`ControllerInput`].
        #[doc = $docstring]
        $camel_case_name,
    )+
    /// Special variable that requests all available variables from the navigator.
    AllAvailable,
}

impl ControllerInputVariable {
    /// Returns the string representation of the variable.
    pub fn to_string(&self) -> String {
        match self {
            $(
                ControllerInputVariable::$camel_case_name => stringify!($camel_case_name).to_string(),
            )+
            ControllerInputVariable::AllAvailable => "AllAvailable".to_string(),
        }
    }

    /// Returns a vector of all available controller input variables except [`ControllerInputVariable::AllAvailable`].
    pub fn all() -> &'static [ControllerInputVariable] {
        &[
            $(
                ControllerInputVariable::$camel_case_name,
            )+
        ]
    }
}

/// Struct filled with variables requested by the controller from the navigator.
/// Each field corresponds to a specific variable in [`ControllerInputVariable`].
#[derive(Debug, Clone, Default)]
pub struct ControllerInputVariables {
    $(
        /// Variable corresponding to the `$camel_case_name` field in [`ControllerInputVariable`].
        #[doc = $docstring]
        pub $snake_case_name: Option<$t>,
    )+
}

impl Recordable<ControllerInputRecord> for ControllerInputVariables {
    fn record(&self, context: &Context) -> ControllerInputRecord {
        ControllerInputRecord {
            $(
                $snake_case_name: self.$snake_case_name.as_ref().map(|o| o.record(context)),
            )+
        }
    }
}

/// Struct filled with variables requested by the controller from the navigator.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ControllerInputRecord {
    $(
        /// Variable corresponding to the `$camel_case_name` field in [`ControllerInputVariable`].
        #[doc = $docstring]
        pub $snake_case_name: Option<$t_record>,
    )+
}

#[cfg(feature = "gui")]
impl UIComponent for ControllerInputRecord {
    fn show(&self, ui: &mut egui::Ui, _ctx: &egui::Context, _unique_id: &str) {
        ui.vertical(|ui| {
            $(
                if let Some(value) = &self.$snake_case_name {
                    ui.horizontal(|ui| {
                        ui.label(stringify!($snake_case_name));
                        ui.label(format!("{:?}", value));
                    });
                }
            )+
        });
    }
}


/****************************************************************************************************
 * ControllerInput affiliated macros
 ***************************************************************************************************/

macro_rules! _generate_declare_controller_input {
    ($d:tt) => {
/// Helper macro to declare a controller input struct using a subset of the variables defined in [`ControllerInputVariable`].
///
/// This macros also generates the code to convert from [`ControllerInputVariables`] to the declared struct, and implements
/// the [`ControllerInput`](crate::controllers::ControllerInput) trait for the struct.
///
/// A panic will occur if a variable is requested that is not provided by the navigator (if it is None in the [`ControllerInputVariables`] instance).
#[macro_export]
macro_rules! declare_controller_input {
    ($d (#[$d meta:meta])*  // Capture attributes including doc comments
    $d name: ident;
    $d (
        $d var_name: ident
    ),+ $d (,)?) => {

/// Helper macro to declare a controller input variable type of the specified variable
macro_rules! _controller_input_get_type {
    $(
    ($snake_case_name) => {$t};
    )+
}
macro_rules! _controller_input_docstring {
    $(
    ($snake_case_name) => {$docstring};
    )+
}
$d (#[$d meta])*
#[derive(Debug, Clone)]
pub struct $d name {
    $d (
    #[doc = _controller_input_docstring!($d var_name)]
    pub $d var_name: _controller_input_get_type!($d var_name),
    )+
}

/// Helper macro to get the corresponding enum variant for a given snake_case variable name.
macro_rules! _controller_input_get_camel_case_with_enum_name {
    $(
    ($snake_case_name) => {$crate::interfaces::navigator_controller::ControllerInputVariable::$camel_case_name};
    )+
}

impl $crate::controllers::ControllerInput for $d name {
    fn required_variables() -> &'static [$crate::interfaces::navigator_controller::ControllerInputVariable] {
        &[
        $d(
            _controller_input_get_camel_case_with_enum_name!($d var_name),
        )+
        ]
    }
}

impl From<$crate::interfaces::navigator_controller::ControllerInputVariables> for $d name {
    fn from(variables: $crate::interfaces::navigator_controller::ControllerInputVariables) -> Self {
        $d name {
        $d(
            $d var_name: variables.$d var_name.expect(concat!("Controller variable `", stringify!($d var_name), "` is required for `", stringify!($d name), "` but is not provided by the navigator.")),
        )+
        }
    }
}
}; // End of general case
    (
        $d (#[$d meta:meta])*  // Capture attributes including doc comments
        $d name: ident
    ) => { // Special case for no variables: all availables
#[derive(Debug, Clone)]
$d (#[$d meta])*
pub struct $d name {
    $(
    #[doc = $docstring]
    pub $snake_case_name: Option<$t>,
    )+
}

impl $crate::controllers::ControllerInput for $d name {
    fn required_variables() -> &'static [$crate::interfaces::navigator_controller::ControllerInputVariable] {
        &[
            $crate::interfaces::navigator_controller::ControllerInputVariable::AllAvailable,
        ]
    }
}

impl From<$crate::interfaces::navigator_controller::ControllerInputVariables> for $d name {
    fn from(variables: $crate::interfaces::navigator_controller::ControllerInputVariables) -> Self {
        $d name {
        $(
            $snake_case_name: variables.$snake_case_name,
        )+
        }
    }
}

impl From<$d name> for $crate::interfaces::navigator_controller::ControllerInputVariables {
    fn from(s: $d name) -> Self {
        Self {
        $(
            $snake_case_name: s.$snake_case_name,
        )+
        }
    }
}

macro_rules! _make_controller_input_python_wrapper {
    ($c: expr) => {
        paste::paste!{
#[pyclass(get_all, set_all)]
#[pyo3(name = "ControllerInput")]
#[derive(Debug, Clone, Default)]
pub struct [<$d name Wrapper>] {
    $(
    #[doc = $docstring]
    pub $snake_case_name: Option<$t_python>,
    )+
}

#[pymethods]
impl [<$d name Wrapper>] {
    /// Create a zero-initialized controller input.
    #[new]
    pub fn new() -> Self {
        Self::default()
    }
}

impl From<&$d name> for [<$d name Wrapper>] {
    fn from(s: &$d name) -> Self {
        Self {
            $(
                $snake_case_name: s.$snake_case_name.clone().map(|v| v.into()),
            )+
        }
    }
}
// crate => called by simusa-py
$c::general::impl_from_plain_with_from_ref!([<$d name Wrapper>], $d name);

impl From<&[<$d name Wrapper>]> for $d name {
    fn from(s: &[<$d name Wrapper>]) -> Self {
        Self {
            $(
                $snake_case_name: s.$snake_case_name.clone().map(|v| v.into()),
            )+
        }
    }
}
// crate => called by simusa-py
$c::general::impl_from_plain_with_from_ref!($d name, [<$d name Wrapper>]);
        }
    };
}
pub(crate) use _make_controller_input_python_wrapper;
} // End of special case for no variables
    } // End of declare_controller_input
}
    }// End of _generate_declare_controller_input
_generate_declare_controller_input!($);
pub use declare_controller_input;


/****************************************************************************************************
 * NavigatorOutput macros
 ***************************************************************************************************/

macro_rules! _generate_declare_navigator_output {
    ($d:tt) => {
#[macro_export]
/// Declares a new navigator output struct with the specified variables,
/// implementing the [`NavigatorOutput`](crate::navigators::NavigatorOutput) trait and
/// conversion to [`ControllerInputVariables`].
///
/// Using this macro enforce that the given variables exists.
macro_rules! declare_navigator_output {
    ($d (#[$d meta:meta])*  // Capture attributes including doc comments
    $d name: ident;
    $d (
        $d var_name: ident
    ),+ $d (,)?) => {

/// Helper macro to declare a navigator output variable type of the specified variable
macro_rules! _navigator_output_get_type {
    $(
    ($snake_case_name) => {$t};
    )+
}
macro_rules! _navigator_output_docstring {
    $(
    ($snake_case_name) => {$docstring};
    )+
}
$d (#[$d meta])*
#[derive(Debug, Clone)]
pub struct $d name {
    $d (
    #[doc = _navigator_output_docstring!($d var_name)]
    pub $d var_name: _navigator_output_get_type!($d var_name),
    )+
}

/// Helper macro to get the corresponding enum variant for a given snake_case variable name.
macro_rules! _navigator_output_get_camel_case_with_enum_name {
    $(
    ($snake_case_name) => {$crate::interfaces::navigator_controller::ControllerInputVariable::$camel_case_name};
    )+
}

impl $crate::navigators::NavigatorOutput for $d name {
    fn available_variables() -> &'static [$crate::interfaces::navigator_controller::ControllerInputVariable] {
        &[
        $d(
            _navigator_output_get_camel_case_with_enum_name!($d var_name),
        )+
        ]
    }

    fn into_controller_input_variables(self, _requested_variables: &[$crate::interfaces::navigator_controller::ControllerInputVariable]) -> $crate::interfaces::navigator_controller::ControllerInputVariables {
        $crate::interfaces::navigator_controller::ControllerInputVariables {
        $d(
            $d var_name: Some(self.$d var_name),
        )+
            ..Default::default()
        }
    }
}
    }
}
pub use declare_navigator_output;
    }
}
_generate_declare_navigator_output!($);

// END
    }
}

generate_controller_input! {
    NodePosition, node_position, (f32, f32), (f32, f32), (f32, f32), "Estinated position of the node in the world frame (x, y).";
    NodeOrientation, node_orientation, f32, f32, f32, "Estimated orientation of the node in the world frame (yaw).";
    NodeLinearVelocity, node_linear_velocity, f32, f32, f32, "Estimated linear velocity of the node in the world frame (m/s).";
    NodeLongitudinalLateralVelocities, node_longitudinal_lateral_velocities, (f32, f32), (f32, f32), (f32, f32), "Estimated longitudinal and lateral velocities of the node in the world frame (m/s).";
    NodeAngularVelocity, node_angular_velocity, f32, f32, f32, "Estimated angular velocity of the node in the world frame (rad/s).";
    WorldState, world_state, WorldState, WorldStateRecord, WorldStateWrapper, "Full estimated state of the world, including all nodes and map.";
    TargetPosition, target_position, (f32, f32), (f32, f32), (f32, f32), "Target position of the node in the world frame (x, y).";
    TargetOrientation, target_orientation, f32, f32, f32, "Target orientation of the node in the world frame (yaw).";
    TargetLinearVelocity, target_linear_velocity, f32, f32, f32, "Target linear velocity of the node in the world frame (m/s).";
    TargetLongitudinalLateralVelocities, target_longitudinal_lateral_velocities, (f32, f32), (f32, f32), (f32, f32), "Target longitudinal and lateral velocities of the node in the world frame (m/s).";
    TargetAngularVelocity, target_angular_velocity, f32, f32, f32, "Target angular velocity of the node in the world frame (rad/s).";
    ErrorLongitudinal, error_longitudinal, f32, f32, f32, "Error in the longitudinal direction (m).";
    ErrorLateral, error_lateral, f32, f32, f32, "Error in the lateral direction (m).";
    ErrorAngular, error_angular, f32, f32, f32, "Error in the angular direction (rad).";
    ErrorLinearVelocity, error_linear_velocity, f32, f32, f32, "Error in the linear velocity (m/s).";
    ErrorLongitudinalLateralVelocities, error_longitudinal_lateral_velocities, (f32, f32), (f32, f32), (f32, f32), "Error in the longitudinal and lateral velocities (m/s).";
    ErrorAngularVelocity, error_angular_velocity, f32, f32, f32, "Error in the angular velocity (rad/s).";
    Custom, custom, GeneralValue, GeneralValue, String, "Custom variable that can be used to send any type of data from the navigator to the controller (when both are user-defined).";
}
