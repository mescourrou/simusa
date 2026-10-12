/*!
Module providing the trait to link the simulator to external implementations.

Example to use an external state estimator:
```ignore
use simusa::state_estimators::state_estimator::StateEstimator;
use simusa::{plugin_api::PluginAPI, simulator::SimulatorConfig};

pub struct MyPlugin;

impl PluginAPI for MyPlugin {
    fn get_state_estimator(
        &self,
        config: &serde_json::Value,
        global_config: &SimulatorConfig,
        va_factory: &Arc<DeterministRandomVariableFactory>,
        network: &SharedRwLock<Network>,
        initial_time: f32,
        context: &Context,
    ) -> Box<dyn StateEstimator> {
        Box::new(MyFilter::from_config(
            &serde_json::from_value(config.value.clone())
                .expect("Error during parsing MyFilter config"),
        ))
    }
}
// You should use the simulator as a library. Your main.rs could be:

use simusa::simulator::Simulator;
use std::path::Path;

fn main() {
    let plugin = MyPlugin {};

    let config_path = Path::new("configs/simulator_config.yaml");
    let mut simulator = Simulator::from_config_path(
        config_path,
        Some(Arc::new(plugin)),
        );

    simulator.run();


}
```
*/

use std::sync::Arc;

use crate::{
    context::Context,
    controllers::AbstractController,
    navigators::AbstractNavigator,
    networking::network::Network,
    physics::{Physics, fault_models::PhysicsFaultModel},
    result_analyser::ResultAnalyserTrait,
    sensors::{Sensor, fault_models::FaultModel, sensor_filters::SensorFilter},
    simulator::SimulatorConfig,
    state_estimators::StateEstimator,
    utils::{SharedRwLock, determinist_random_variable::DeterministRandomVariableFactory},
};

/// Trait to link the simulator to the external implementation.
#[allow(unused_variables)]
pub trait PluginAPI: Send + Sync {
    /// Return the [`StateEstimator`] to be used by the
    /// [`PluginEstimator`](`crate::state_estimators::external_estimator::PluginEstimator`).
    ///
    /// # Arguments
    /// * `config` - Config for the external state estimator. The configuration
    ///   is given using [`serde_json::Value`]. It should be converted by the
    ///   external plugin to the specific configuration.
    /// * `global_config` - Full configuration of the simulator.
    /// * `va_factory` - Factory for Determinists random variables to create random variables.
    /// * `network` - Reference to the network, to allow the state estimator to send messages or create channels.
    /// * `initial_time` - Initial time of the simulation (or the time of creation).
    /// * `context` - Context for the simulation, to allow the state estimator to log.
    ///
    /// # Return
    ///
    /// Returns the [`StateEstimator`] to use.
    fn get_state_estimator(
        &self,
        config: &serde_json::Value,
        global_config: &SimulatorConfig,
        va_factory: &Arc<DeterministRandomVariableFactory>,
        network: &SharedRwLock<Network>,
        initial_time: f32,
        context: &Context,
    ) -> Box<dyn StateEstimator> {
        panic!("The given PluginAPI does not provide a state estimator");
    }

    /// Return the [`Controller`] to be used by the
    /// [`PluginController`](`crate::controllers::external_controller::PluginController`).
    ///
    /// # Arguments
    /// * `config` - Config for the external controller. The configuration
    ///   is given using [`serde_json::Value`]. It should be converted by the
    ///   external plugin to the specific configuration.
    /// * `global_config` - Full configuration of the simulator.
    /// * `va_factory` - Factory for Determinists random variables to create random variables.
    /// * `network` - Reference to the network, to allow the controller to send messages or create channels.
    /// * `initial_time` - Initial time of the simulation (or the time of creation).
    /// * `context` - Context for the simulation, to allow the controller to log.
    ///
    /// # Return
    ///
    /// Returns the [`Controller`] to use.
    fn get_controller(
        &self,
        config: &serde_json::Value,
        global_config: &SimulatorConfig,
        va_factory: &Arc<DeterministRandomVariableFactory>,
        network: &SharedRwLock<Network>,
        initial_time: f32,
        context: &Context,
    ) -> Box<dyn AbstractController> {
        panic!("The given PluginAPI does not provide a controller");
    }

    /// Return the [`Navigator`] to be used by the
    /// [`PluginNavigator`](`crate::navigators::external_navigator::PluginNavigator`).
    ///
    /// # Arguments
    /// * `config` - Config for the external navigator. The configuration
    ///   is given using [`serde_json::Value`]. It should be converted by the
    ///   external plugin to the specific configuration.
    /// * `global_config` - Full configuration of the simulator.
    /// * `va_factory` - Factory for Determinists random variables to create random variables.
    /// * `network` - Reference to the network, to allow the navigator to send messages or create channels.
    /// * `initial_time` - Initial time of the simulation (or the time of creation).
    /// * `context` - Context for the simulation, to allow the navigator to log.
    ///
    /// # Return
    ///
    /// Returns the [`Navigator`] to use.
    fn get_navigator(
        &self,
        config: &serde_json::Value,
        global_config: &SimulatorConfig,
        va_factory: &Arc<DeterministRandomVariableFactory>,
        network: &SharedRwLock<Network>,
        initial_time: f32,
        context: &Context,
    ) -> Box<dyn AbstractNavigator> {
        panic!("The given PluginAPI does not provide a navigator");
    }

    /// Return the [`Physics`] to be used by the
    /// [`PluginPhysics`](`crate::physics::external_physics::PluginPhysics`).
    ///
    /// # Arguments
    /// * `config` - Config for the external physics. The configuration
    ///   is given using [`serde_json::Value`]. It should be converted by the
    ///   external plugin to the specific configuration.
    /// * `global_config` - Full configuration of the simulator.
    /// * `va_factory` - Factory for Determinists random variables to create random variables.
    /// * `network` - Reference to the network, to allow the physics to send messages or create channels.
    /// * `initial_time` - Initial time of the simulation (or the time of creation).
    /// * `context` - Context for the simulation, to allow the physics to log.
    ///
    /// # Return
    ///
    /// Returns the [`Physics`] to use.
    fn get_physics(
        &self,
        config: &serde_json::Value,
        global_config: &SimulatorConfig,
        va_factory: &Arc<DeterministRandomVariableFactory>,
        network: &SharedRwLock<Network>,
        initial_time: f32,
        context: &Context,
    ) -> Box<dyn Physics> {
        panic!("The given PluginAPI does not provide physics");
    }

    /// Allow the plugin to check for requests from the simulator and react to them.
    /// This is used at the configuration loading step to allow asynchronous plugins to check for requests, especially [`PythonAPI`](crate::pybinds::PythonAPI).
    fn check_requests(&self, context: &Context) {}

    /// Return the [`Sensor`] to be used by the
    /// [`PluginSensor`](`crate::sensors::external_sensor::PluginSensor`).
    ///
    /// # Arguments
    /// * `config` - Config for the external sensor. The configuration
    ///   is given using [`serde_json::Value`]. It should be converted by the
    ///   external plugin to the specific configuration.
    /// * `global_config` - Full configuration of the simulator.
    /// * `_va_factory` - Factory for Determinists random variables to create random variables if needed.
    /// * `network` - Reference to the network, to allow the sensor to send messages if needed.
    /// * `initial_time` - Initial time of the simulation, to allow the sensor to initialize itself with the correct time.
    /// * `context` - Context for the simulation, to allow the sensor to log.
    /// # Return
    /// Returns the [`Sensor`] to use.
    fn get_sensor(
        &self,
        config: &serde_json::Value,
        global_config: &SimulatorConfig,
        va_factory: &Arc<DeterministRandomVariableFactory>,
        network: &SharedRwLock<Network>,
        initial_time: f32,
        context: &Context,
    ) -> Box<dyn Sensor> {
        panic!("The given PluginAPI does not provide a sensor");
    }

    /// Return a list of [`Drawable`](crate::gui::Drawable) to be used by the GUI.
    ///
    /// The GUI will call this function at startup to load an instance of the [`Drawable`](crate::gui::Drawable) to be used for the visualization.
    /// The GUI will then call the [`Drawable::add_robot_record`](crate::gui::Drawable::add_robot_record), [`Drawable::add_computation_unit_record`](crate::gui::Drawable::add_computation_unit_record),
    /// [`Drawable::add_map_record`](crate::gui::Drawable::add_map_record), and [`Drawable::add_scenario_record`](crate::gui::Drawable::add_scenario_record) functions (depending on the type of the record)
    /// at each step of the simulation to update the drawable with the new record.
    /// The GUI will then call [`Drawable::draw`](crate::gui::Drawable::draw) and [`Drawable::react`](crate::gui::Drawable::react) function at each frame to draw the drawable and react to click.
    #[cfg(feature = "gui")]
    fn get_drawables(&self, global_config: &SimulatorConfig) -> Vec<Box<dyn crate::gui::Drawable>> {
        Vec::new()
    }

    /// Return the [`SensorFilter`] to be used by the
    /// [`PluginFilter`](`crate::sensors::sensor_filters::external_filter::PluginFilter`).
    ///
    /// # Arguments
    /// * `config` - Config for the external sensor filter. The configuration
    ///   is given using [`serde_json::Value`]. It should be converted by the
    ///   external plugin to the specific configuration.
    /// * `global_config` - Full configuration of the simulator.
    /// * `va_factory` - Factory for Determinists random variables.
    /// * `initial_time` - Initial time of the simulation.
    /// * `context` - Context for the simulation, to allow the sensor filter to log.
    ///
    /// # Return
    ///
    /// Returns the [`SensorFilter`] to use.
    fn get_sensor_filter(
        &self,
        config: &serde_json::Value,
        global_config: &SimulatorConfig,
        va_factory: &Arc<DeterministRandomVariableFactory>,
        initial_time: f32,
        context: &Context,
    ) -> Box<dyn SensorFilter> {
        panic!("The given PluginAPI does not provide a sensor filter");
    }

    /// Return the [`FaultModel`] to be used by the
    /// [`PluginFault`](`crate::sensors::fault_models::external_fault::PluginFault`).
    ///
    /// # Arguments
    /// * `config` - Config for the external sensor fault. The configuration
    ///   is given using [`serde_json::Value`]. It should be converted by the
    ///   external plugin to the specific configuration.
    /// * `global_config` - Full configuration of the simulator.
    /// * `va_factory` - Factory for Determinists random variables.
    /// * `initial_time` - Initial time of the simulation.
    /// * `context` - Context for the simulation, to allow the sensor fault to log.
    ///
    /// # Return
    ///
    /// Returns the [`FaultModel`] to use.
    fn get_sensor_fault(
        &self,
        config: &serde_json::Value,
        global_config: &SimulatorConfig,
        va_factory: &Arc<DeterministRandomVariableFactory>,
        initial_time: f32,
        context: &Context,
    ) -> Box<dyn FaultModel> {
        panic!("The given PluginAPI does not provide a sensor fault model");
    }

    /// Return the [`PhysicsFaultModel`] to be used by the
    /// [`InternalPhysics`](`crate::physics::internal_physics::InternalPhysics`).
    ///
    /// # Arguments
    /// * `config` - Config for the external sensor fault. The configuration
    ///   is given using [`serde_json::Value`]. It should be converted by the
    ///   external plugin to the specific configuration.
    /// * `global_config` - Full configuration of the simulator.
    /// * `va_factory` - Factory for Determinists random variables.
    /// * `initial_time` - Initial time of the simulation.
    /// * `context` - Context for the simulation, to allow the sensor fault to log.
    ///
    /// # Return
    ///
    /// Returns the [`FaultModel`] to use.
    fn get_physics_fault(
        &self,
        config: &serde_json::Value,
        global_config: &SimulatorConfig,
        va_factory: &Arc<DeterministRandomVariableFactory>,
        initial_time: f32,
        context: &Context,
    ) -> Box<dyn PhysicsFaultModel> {
        panic!("The given PluginAPI does not provide a physics fault model");
    }

    /// Return the [`ResultAnalyserTrait`] to be used by the [`ResultAnalyser`](`crate::result_analyser::ResultAnalyser`).
    ///
    /// # Arguments
    /// * `config` - Config for the external result analyser. The configuration
    ///   is given using [`serde_json::Value`]. It should be converted by the
    ///   external plugin to the specific configuration.
    /// * `global_config` - Full configuration of the simulator.
    /// * `context` - Context for the simulation, to allow the result analyser to log.
    ///
    /// # Return
    ///
    /// Returns the [`ResultAnalyserTrait`] to use.
    fn get_result_analyser(
        &self,
        config: &serde_json::Value,
        global_config: &SimulatorConfig,
        context: &Context,
    ) -> Box<dyn ResultAnalyserTrait> {
        panic!("The given PluginAPI does not provide a result analyser");
    }
}
