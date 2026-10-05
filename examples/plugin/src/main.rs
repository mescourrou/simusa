#[allow(unused_variables)]
use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use simusa::context::Context;
use simusa::controllers::{Controller, ControllerError, ControllerRecord};
use simusa::errors::SimusaResult;
use simusa::info;
use simusa::navigators::{Navigator, NavigatorRecord};
use simusa::networking::network::Network;
use simusa::node::Node;
use simusa::physics::robot_models::unicycle::UnicycleCommand;
use simusa::physics::robot_models::Command;
use simusa::physics::{Physics, PhysicsRecord};
use simusa::plugin_api::PluginAPI;
use simusa::recordable::Recordable;
use simusa::sensors::{Observation, Sensor, SensorObservation, SensorRecord};
use simusa::simulator::{Simulator, SimulatorConfig};
use simusa::state_estimators::{
    State, StateEstimator, StateEstimatorRecord, StateRecord, WorldState,
};
use simusa::utils::determinist_random_variable::DeterministRandomVariableFactory;
use simusa::utils::frame::Frame;
use simusa::utils::SharedRwLock;
use std::path::Path;
use std::sync::Arc;

///////////////////////////////////
/*     CONTROLLER TEMPLATE       */
///////////////////////////////////

#[derive(Debug, Serialize, Deserialize)]
struct MyWonderfulControllerRecord {}

#[derive(Debug, Serialize, Deserialize)]
struct MyWonderfulControllerConfig {}

#[derive(Debug)]
struct MyWonderfulController {}

impl MyWonderfulController {
    pub fn from_config(_config: MyWonderfulControllerConfig, _initial_time: f32) -> Self {
        Self {}
    }
}

impl Controller for MyWonderfulController {
    fn make_command(
        &mut self,
        _robot: &mut simusa::node::Node,
        _error: &simusa::controllers::ControllerError,
        _time: f32,
        _context: &Context,
    ) -> Command {
        Command::Unicycle(UnicycleCommand {
            left_wheel_speed: 0.,
            right_wheel_speed: 0.,
        })
    }

    fn pre_loop_hook(&mut self, _node: &mut simusa::node::Node, _time: f32, _context: &Context) {}
}

impl Recordable<ControllerRecord> for MyWonderfulController {
    fn record(&self, _context: &Context) -> ControllerRecord {
        ControllerRecord::Plugin(serde_json::to_value(MyWonderfulControllerRecord {}).unwrap())
    }
}

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

impl Navigator for MyWonderfulNavigator {
    fn compute_error(
        &mut self,
        _robot: &mut simusa::node::Node,
        _state: WorldState,
        _context: &Context,
    ) -> ControllerError {
        ControllerError {
            lateral: 0.,
            theta: 0.,
            velocity: 0.,
            longitudinal: 0.,
        }
    }

    fn pre_loop_hook(&mut self, _node: &mut simusa::node::Node, _time: f32, _context: &Context) {}
}

impl Recordable<NavigatorRecord> for MyWonderfulNavigator {
    fn record(&self, _context: &Context) -> NavigatorRecord {
        NavigatorRecord::Plugin(serde_json::to_value(MyWonderfulNavigatorRecord {}).unwrap())
    }
}

///////////////////////////////////
/// PHYSICS TEMPLATE
///////////////////////////////////

#[derive(Debug, Serialize, Deserialize)]
struct MyWonderfulPhysicsRecord {
    state: StateRecord,
}

#[derive(Debug, Serialize, Deserialize)]
struct MyWonderfulPhysicsConfig {}

#[derive(Debug)]
struct MyWonderfulPhysics {
    state: State,
}

impl MyWonderfulPhysics {
    pub fn from_config(_config: MyWonderfulPhysicsConfig, _initial_time: f32) -> Self {
        Self {
            state: State {
                pose: Vector3::zeros(),
                velocity: Vector3::zeros(),
            },
        }
    }
}

impl Physics for MyWonderfulPhysics {
    fn apply_command(&mut self, _command: &Command, _time: f32, _context: &Context) {}

    fn state(&self, _time: f32, _context: &Context) -> State {
        self.state.clone()
    }

    fn update_state(&mut self, _time: f32, _context: &Context) {}
}

impl Recordable<PhysicsRecord> for MyWonderfulPhysics {
    fn record(&self, context: &Context) -> PhysicsRecord {
        PhysicsRecord::Plugin(serde_json::to_value(MyWonderfulPhysicsRecord {
                state: self.state.record(context),
            })
            .unwrap()
        )
    }
}

///////////////////////////////////
/// STATE ESTIMATOR TEMPLATE
///////////////////////////////////

#[derive(Debug, Serialize, Deserialize)]
struct MyWonderfulStateEstimatorRecord {
    pub last_prediction: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct MyWonderfulStateEstimatorConfig {}

#[derive(Debug)]
struct MyWonderfulStateEstimator {
    last_prediction: f32,
}

impl MyWonderfulStateEstimator {
    pub fn from_config(_config: MyWonderfulStateEstimatorConfig, initial_time: f32) -> Self {
        Self {
            last_prediction: initial_time,
        }
    }
}

impl StateEstimator for MyWonderfulStateEstimator {
    fn prediction_step(
        &mut self,
        _robot: &mut simusa::node::Node,
        _command: Option<Command>,
        time: f32,
        _context: &Context,
    ) {
        self.last_prediction = time;
    }

    fn correction_step(
        &mut self,
        _robot: &mut simusa::node::Node,
        _observations: &[Observation],
        _time: f32,
        _context: &Context,
    ) {
    }

    fn next_time_step(&self, _context: &Context) -> f32 {
        self.last_prediction + 0.5
    }

    fn world_state(&self, _context: &Context) -> WorldState {
        WorldState::new()
    }

    fn pre_loop_hook(&mut self, _node: &mut simusa::node::Node, _time: f32, _context: &Context) {}
}

impl Recordable<StateEstimatorRecord> for MyWonderfulStateEstimator {
    fn record(&self, _context: &Context) -> StateEstimatorRecord {
        StateEstimatorRecord::Plugin(
            serde_json::to_value(MyWonderfulStateEstimatorRecord {
                last_prediction: self.last_prediction,
            })
            .unwrap()
        )
    }
}

///////////////////////////////////
/// SENSOR TEMPLATE
///////////////////////////////////

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MyWonderfulSensorObservation {
    pub data: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct MyWonderfulSensorConfig {
    pub period: f32,
}

#[derive(Debug)]
struct MyWonderfulSensor {
    last_time: f32,
    period: f32,
    last_observation: Option<MyWonderfulSensorObservation>,
}

impl MyWonderfulSensor {
    pub fn from_config(config: MyWonderfulSensorConfig, initial_time: f32) -> Self {
        Self {
            last_time: initial_time,
            period: config.period,
            last_observation: None,
        }
    }
}

impl Sensor for MyWonderfulSensor {
    fn post_init(
        &mut self,
        node: &mut Node,
        _initial_time: f32,
        context: &Context,
    ) -> SimusaResult<()> {
        info!(
            context,
            "Initializing MyWonderfulSensor for node {}",
            node.name()
        );
        Ok(())
    }

    fn get_observations(
        &mut self,
        _node: &mut Node,
        time: f32,
        _frame: &Frame,
        _context: &Context,
    ) -> Vec<SensorObservation> {
        self.last_observation = Some(MyWonderfulSensorObservation { data: time });
        self.last_time = time;
        // Return a custom observation here, but you can return an existing one as well (e.g. SpeedObservation)
        vec![SensorObservation::Plugin(serde_json::to_value(self.last_observation.as_ref().unwrap()).unwrap())]
    }

    fn next_time_step(&self) -> f32 {
        self.last_time + self.period
    }
}

impl Recordable<SensorRecord> for MyWonderfulSensor {
    fn record(&self, _context: &Context) -> SensorRecord {
        SensorRecord::Plugin(serde_json::to_value(self.last_observation.clone()).unwrap())
    }
}

struct MyWonderfulPlugin {}

impl PluginAPI for MyWonderfulPlugin {
    fn get_controller(
        &self,
        config: &serde_json::Value,
        _global_config: &SimulatorConfig,
        _va_factory: &Arc<DeterministRandomVariableFactory>,
        _network: &SharedRwLock<Network>,
        initial_time: f32,
        _context: &Context,
    ) -> Box<dyn Controller> {
        Box::new(MyWonderfulController::from_config(
            serde_json::from_value(config.clone()).unwrap(),
            initial_time,
        ))
    }

    fn get_navigator(
        &self,
        config: &serde_json::Value,
        _global_config: &SimulatorConfig,
        _va_factory: &Arc<DeterministRandomVariableFactory>,
        _network: &SharedRwLock<Network>,
        initial_time: f32,
        _context: &Context,
    ) -> Box<dyn Navigator> {
        Box::new(MyWonderfulNavigator::from_config(
            serde_json::from_value(config.clone()).unwrap(),
            initial_time,
        ))
    }

    fn get_physics(
        &self,
        config: &serde_json::Value,
        _global_config: &SimulatorConfig,
        _va_factory: &Arc<DeterministRandomVariableFactory>,
        _network: &SharedRwLock<Network>,
        initial_time: f32,
        _context: &Context,
    ) -> Box<dyn Physics> {
        Box::new(MyWonderfulPhysics::from_config(
            serde_json::from_value(config.clone()).unwrap(),
            initial_time,
        ))
    }

    fn get_state_estimator(
        &self,
        config: &serde_json::Value,
        _global_config: &SimulatorConfig,
        _va_factory: &Arc<DeterministRandomVariableFactory>,
        _network: &SharedRwLock<Network>,
        initial_time: f32,
        _context: &Context,
    ) -> Box<dyn StateEstimator> {
        Box::new(MyWonderfulStateEstimator::from_config(
            serde_json::from_value(config.clone()).unwrap(),
            initial_time,
        ))

        // Example: use already existing state estimator (PythonEstimator here)
        // let config = serde_json::from_value(config.clone()).unwrap();
        // Box::new(PythonEstimator::from_config(&config, global_config).unwrap())
    }

    fn get_sensor(
        &self,
        config: &serde_json::Value,
        _global_config: &SimulatorConfig,
        _va_factory: &Arc<DeterministRandomVariableFactory>,
        _network: &SharedRwLock<Network>,
        initial_time: f32,
        _context: &Context,
    ) -> Box<dyn Sensor> {
        Box::new(MyWonderfulSensor::from_config(
            serde_json::from_value(config.clone()).unwrap(),
            initial_time,
        ))
    }
}

fn main() {
    let my_plugin = MyWonderfulPlugin {};

    println!("Load configuration...");
    let mut simulator = Simulator::from_config_path(
        Path::new("config_example/config_plugin.yaml"),
        Some(Arc::new(my_plugin)), //<- plugin API, to load external modules
    )
    .unwrap();

    println!("------------------ RUN TO 10");
    simulator.set_max_time(10.);
    // Run the simulator for the time given in the configuration
    // It also save the results to json
    simulator.run().unwrap();
    println!("{:?}", simulator.get_records(false));

    println!("------------------ RUN TO 20");
    simulator.set_max_time(20.);
    // Run the simulator for the time given in the configuration
    // It also save the results to json
    simulator.run().unwrap();
    println!("{:?}", simulator.get_records(false));

    simulator.compute_results().unwrap();
}
