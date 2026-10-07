use egui::Rect;
#[allow(unused_variables)]
use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use simusa::context::Context;
use simusa::controllers::{Controller, ControllerError, ControllerRecord};
use simusa::errors::SimusaResult;
use simusa::gui::Drawable;
use simusa::info;
use simusa::navigators::{Navigator, NavigatorRecord};
use simusa::networking::network::Network;
use simusa::node::Node;
use simusa::physics::fault_models::PhysicsFaultModel;
use simusa::physics::robot_models::unicycle::UnicycleCommand;
use simusa::physics::robot_models::Command;
use simusa::physics::{Physics, PhysicsRecord};
use simusa::plugin_api::PluginAPI;
use simusa::recordable::Recordable;
use simusa::result_analyser::ResultAnalyserTrait;
use simusa::sensors::fault_models::FaultModel;
use simusa::sensors::sensor_filters::SensorFilter;
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

///////////////////////////////////
/// PHYSICS FAULT TEMPLATE
///////////////////////////////////

#[derive(Debug, Serialize, Deserialize)]
struct MyWonderfulPhysicsFaultConfig {}

#[derive(Debug)]
pub struct MyWonderfulPhysicsFault {}

impl MyWonderfulPhysicsFault {
    pub fn from_config(_config: MyWonderfulPhysicsFaultConfig, _initial_time: f32) -> Self {
        Self {}
    }
}

impl PhysicsFaultModel for MyWonderfulPhysicsFault {
    fn post_init(&mut self, _node: &mut simusa::node::Node, context: &Context) -> SimusaResult<()> {
        Ok(())
    }

    fn add_faults(&self, time: f32, state: &mut State, context: &Context) {
        state.pose.x += 0.1;
    }
}

///////////////////////////////////
/// SENSOR FAULT TEMPLATE
///////////////////////////////////

#[derive(Debug, Serialize, Deserialize)]
struct MyWonderfulSensorFaultConfig {}

#[derive(Debug)]
pub struct MyWonderfulSensorFault {}

impl MyWonderfulSensorFault {
    pub fn from_config(_config: MyWonderfulSensorFaultConfig, _initial_time: f32) -> Self {
        Self {}
    }
}

impl FaultModel for MyWonderfulSensorFault {
    fn post_init(&mut self, _node: &mut simusa::node::Node, _initial_time: f32, context: &Context) -> SimusaResult<()> {
        Ok(())
    }

    fn add_faults(
        &mut self,
        time: f32,
        seed: f32,
        obs_list: &mut Vec<SensorObservation>,
        obs_type: SensorObservation,
        environment: &Arc<simusa::environment::Environment>,
        context: &Context,
    )
    {
        // Add your fault here, for example, remove the last observation
        obs_list.pop();
    }
}

///////////////////////////////////
/// SENSOR FILTER TEMPLATE
///////////////////////////////////

#[derive(Debug, Serialize, Deserialize)]
struct MyWonderfulSensorFilterConfig {}

#[derive(Debug)]
pub struct MyWonderfulSensorFilter {}

impl MyWonderfulSensorFilter {
    pub fn from_config(_config: MyWonderfulSensorFilterConfig, _initial_time: f32) -> Self {
        Self {}
    }
}

impl SensorFilter for MyWonderfulSensorFilter {
    fn post_init(&mut self, _node: &mut simusa::node::Node, initial_time: f32, context: &Context) -> SimusaResult<()> {
        Ok(())
    }

    fn filter(
        &self,
        time: f32,
        observation: SensorObservation,
        observer_state: &State,
        observee_state: Option<&State>,
        context: &Context,
    ) -> Option<SensorObservation>
    {
        // Add you filter here, for example, keep only GNSS
        match observation {
            SensorObservation::GNSS(_) => Some(observation),
            _ => None,
        }
        
    }
}

///////////////////////////////////
/// DRAWABLE TEMPLATE
///////////////////////////////////
pub struct MyWonderfulDrawable {
    thickness: f32,
}

impl Drawable for MyWonderfulDrawable {
    fn draw(
        &self,
        ui: &mut egui::Ui,
        viewport: &egui::Rect,
        painter_info: &simusa::gui::PainterInfo,
        scale: f32,
        time: f32,
    ) -> Result<Vec<egui::Shape>, egui::Vec2>
    {
        Ok(vec![egui::Shape::line_segment(
            [
                egui::pos2(viewport.left(), viewport.top()),
                egui::pos2(viewport.right(), viewport.bottom()),
            ],
            egui::Stroke::new(self.thickness, egui::Color32::from_rgb(255, 0, 0)),
        )])
    }

    fn react(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        response: &egui::Response,
        painter_info: &simusa::gui::PainterInfo,
        scale: f32,
        time: f32,
    )
    {
        // Manage clicks, drags, etc. on the drawable here
    }

    fn name(&self) -> Option<String> {
        Some("Horrible line".to_string())
    }

    fn associated_node_name(&self) -> Option<String> {
        Some("node1".to_string()) // Name of the robot/node if specific to a robot, otherwise None (for global drawables)
    }

    fn show_params_mut(&mut self, ui: &mut egui::Ui, unique_id: &str) {
        // Show parameters of the drawable here, for example, a color picker
        ui.horizontal(|ui| {
            ui.add(egui::Slider::new(&mut self.thickness, 0.0..=10.0).text("Line thickness"));
        });
    }

    // You can choose only one of the four methods below to implement, depending on your needs.
    fn add_computation_unit_record(&mut self, time: f32, record: simusa::node::node_factory::ComputationUnitRecord) {
        
    }

    fn add_map_record(&mut self, time: f32, record: simusa::environment::map::MapRecord) {
        
    }

    fn add_robot_record(&mut self, time: f32, record: simusa::node::node_factory::RobotRecord) {
        
    }

    fn add_scenario_record(&mut self, time: f32, record: simusa::scenario::config::EventRecord) {
        
    }
}

///////////////////////////////////
/// RESULT ANALYSER TEMPLATE
///////////////////////////////////
#[derive(Debug, Serialize, Deserialize)]
struct MyWonderfulResultAnalyserConfig {}

#[derive(Debug)]
pub struct MyWonderfulResultAnalyser {}

impl MyWonderfulResultAnalyser {
    pub fn from_config(_config: MyWonderfulResultAnalyserConfig, _initial_time: f32) -> Self {
        Self {}
    }
}

impl ResultAnalyserTrait for MyWonderfulResultAnalyser {
    fn analyse_results(&mut self, results: &simusa::simulator::Results, context: &Context) -> SimusaResult<()> {
        // Perform the analysis here
        Ok(())
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

    fn get_physics_fault(
        &self,
        config: &serde_json::Value,
        global_config: &SimulatorConfig,
        va_factory: &Arc<DeterministRandomVariableFactory>,
        initial_time: f32,
        context: &Context,
    ) -> Box<dyn simusa::physics::fault_models::PhysicsFaultModel>
    {
        Box::new(MyWonderfulPhysicsFault::from_config(
            serde_json::from_value(config.clone()).unwrap(),
            initial_time,
        ))
    }

    fn get_sensor_fault(
        &self,
        config: &serde_json::Value,
        global_config: &SimulatorConfig,
        va_factory: &Arc<DeterministRandomVariableFactory>,
        initial_time: f32,
        context: &Context,
    ) -> Box<dyn simusa::sensors::fault_models::FaultModel>
    {
        Box::new(MyWonderfulSensorFault::from_config(
            serde_json::from_value(config.clone()).unwrap(),
            initial_time,
        ))
    }

    fn get_sensor_filter(
        &self,
        config: &serde_json::Value,
        global_config: &SimulatorConfig,
        va_factory: &Arc<DeterministRandomVariableFactory>,
        initial_time: f32,
        context: &Context,
    ) -> Box<dyn simusa::sensors::sensor_filters::SensorFilter>
    {
        Box::new(MyWonderfulSensorFilter::from_config(
            serde_json::from_value(config.clone()).unwrap(),
            initial_time,
        ))
    }

    fn get_drawables(&self, global_config: &SimulatorConfig) -> Vec<Box<dyn simusa::gui::Drawable>> {
        vec![Box::new(MyWonderfulDrawable { thickness: 2.0 })]
    }

    fn get_result_analyser(
        &self,
        config: &serde_json::Value,
        global_config: &SimulatorConfig,
        context: &Context,
    ) -> Box<dyn simusa::result_analyser::ResultAnalyserTrait>
    {
        Box::new(MyWonderfulResultAnalyser::from_config(
            serde_json::from_value(config.clone()).unwrap(),
            0.0,
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
