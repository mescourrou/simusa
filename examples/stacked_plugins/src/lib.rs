use serde::{Deserialize, Serialize};
use simusa::context::Context;
use simusa::navigators::{AbstractNavigator, Navigator, NavigatorRecord};
use simusa::networking::network::Network;
use simusa::plugin_api::PluginAPI;
use simusa::recordable::Recordable;
use simusa::simulator::{AsyncSimulator, SimulatorConfig};
use simusa::state_estimators::{StateEstimator, WorldState};
use simusa::utils::{GeneralValue, SharedRwLock};
use simusa::utils::determinist_random_variable::DeterministRandomVariableFactory;
use simusa::{declare_navigator_output, info};
use simusa_py::general::PythonAPI;
use std::sync::Arc;

use pyo3::prelude::*;

///////////////////////////////////
/// NAVIGATOR TEMPLATE
///////////////////////////////////

#[derive(Debug, Serialize, Deserialize)]
struct MyWonderfulNavigatorRecord {}

#[derive(Debug, Serialize, Deserialize)]
struct MyWonderfulNavigatorConfig {}

#[derive(Debug)]
struct MyWonderfulNavigator {}

impl MyWonderfulNavigator {
    pub fn from_config(_config: MyWonderfulNavigatorConfig, _initial_time: f32) -> Self {
        Self {}
    }
}

// Variables that your navigator can compute.
// Make a maximum to maximize compatibility.
declare_navigator_output!(
    MyWonderfulNavigatorOutput;
    // node_position,
    // node_orientation,
    // node_linear_velocity,
    // node_angular_velocity,
    // node_longitudinal_lateral_velocities,
    // target_position,
    // target_orientation,
    // target_linear_velocity,
    // target_angular_velocity,
    // target_longitudinal_lateral_velocities,
    error_longitudinal,
    error_lateral,
    error_angular,
    error_linear_velocity,
    // error_angular_velocity,
    // error_longitudinal_lateral_velocities,
    // world_state,
    // custom // For custom navigator to custom controller
);

impl Navigator for MyWonderfulNavigator {
    type OutputType = MyWonderfulNavigatorOutput;

    fn post_init(
        &mut self,
        _node: &mut simusa::node::Node,
        _context: &Context,
    ) -> simusa::errors::SimusaResult<()> {
        // Optional implementation of post_init, which is called after the navigator is initialized,
        // before the begining of the simulation.
        Ok(())
    }

    fn compute_output(
        &mut self,
        _robot: &mut simusa::node::Node,
        _state: WorldState,
        context: &Context,
    ) -> simusa::errors::SimusaResult<MyWonderfulNavigatorOutput> {
        info!(context, "---Compute error with Rust");
        // Mandatory implementation of precompute_output, which returns a type that implements the [`OutputType`] trait.
        // Use the macro `declare_easy_navigator_output!` or `declare_optimized_navigator_output` to declare the output type.
        Ok(MyWonderfulNavigatorOutput {
            error_longitudinal: 0.,
            error_lateral: 0.,
            error_angular: 0.,
            error_linear_velocity: 0.,
        })
    }

    fn pre_loop_hook(&mut self, _node: &mut simusa::node::Node, _time: f32, _context: &Context) {}

    fn next_time_step(&self, _context: &Context) -> Option<f32> {
        // Optional implementation of next_time_step, which detemines the next time step of the navigator.
        // If None is returned, the simulator will use the state estimator frequency to determine the next time step.
        None
    }
}

impl Recordable<NavigatorRecord> for MyWonderfulNavigator {
    fn record(&self, _context: &Context) -> NavigatorRecord {
        NavigatorRecord::Plugin(serde_json::to_value(MyWonderfulNavigatorRecord {}).unwrap().into())
    }
}

struct MyWonderfulPlugin {
    pub python_api: PythonAPI,
}

impl PluginAPI for MyWonderfulPlugin {
    // Rust Navigator
    fn get_navigator(
        &self,
        config: &GeneralValue,
        _global_config: &SimulatorConfig,
        _va_factory: &Arc<DeterministRandomVariableFactory>,
        _network: &SharedRwLock<Network>,
        initial_time: f32,
        _context: &Context,
    ) -> Box<dyn AbstractNavigator> {
        Box::new(MyWonderfulNavigator::from_config(
            serde_json::from_value(config.to_value().clone()).unwrap(),
            initial_time,
        ))
    }

    // Python state estimator
    fn get_state_estimator(
        &self,
        config: &GeneralValue,
        global_config: &SimulatorConfig,
        va_factory: &Arc<DeterministRandomVariableFactory>,
        network: &SharedRwLock<Network>,
        initial_time: f32,
        context: &Context,
    ) -> Box<dyn StateEstimator> {
        self.python_api.get_state_estimator(
            config,
            global_config,
            va_factory,
            network,
            initial_time,
            context,
        )
    }

    fn check_requests(&self, context: &Context) {
        self.python_api.check_requests(context);
    }
}

#[pymodule]
mod my_rust_plugin {
    #[pymodule_export]
    use super::start;
    #[pymodule_export]
    use simusa_py::simusa;
}

#[pyfunction]
fn start(python_api: Py<PyAny>) {
    let my_plugin = MyWonderfulPlugin {
        python_api: PythonAPI::new(python_api),
    };

    let my_plugin = Some(Arc::new(my_plugin) as Arc<dyn PluginAPI>);

    let mut simulator = match AsyncSimulator::from_config_path("config_plugin.yaml", &my_plugin) {
        Ok(simulator) => simulator,
        Err(e) => {
            panic!("Failed to create simulator: {}", e);
        }
    };
    simulator.run(&my_plugin, Some(20.), false);
    let _ = simulator.get_records(false);
    simulator.run(&my_plugin, Some(40.), false);
    let _ = simulator.get_records(false);
    simulator.compute_results();
    simulator.stop();
}
