use pyo3::prelude::*;
use ::simusa::{navigators::go_to::GoToMessage, networking::network::MessageFlag, plugin_api::PluginAPI, sensors::sensor_manager::SensorTriggerMessage};

use crate::{controllers::ControllerWrapper, navigators::NavigatorWrapper, networking::MessageTypesWrapper, physics::PhysicsWrapper, state_estimators::StateEstimatorWrapper, wrappers::{CommandWrapper, ControllerErrorWrapper, DisplacementObservationWrapper, GNSSObservationWrapper, MultiClientWrapper, NodeWrapper, ObservationWrapper, OrientedLandmarkObservationWrapper, OrientedRobotObservationWrapper, PluginAPIWrapper, Pose, SensorObservationWrapper, SimulatorWrapper, SpeedObservationWrapper, StateWrapper, UnicycleCommandWrapper, Vec2, Vec3, WorldStateWrapper}};


mod controllers;
mod general;
mod navigators;
mod networking;
mod physics;
mod state_estimators;
mod utils;
pub mod wrappers;

/// Run the GUI of the simulator. This function will block until the GUI is closed but
/// will allow communications between Python thread and GUI to be able to use [`PythonAPI`] in the GUI.
#[cfg(feature = "gui")]
#[pyfunction]
#[pyo3(signature = (default_config_path=None, plugin_api=None, load_results=false))]
pub fn run_gui(
    py: Python,
    default_config_path: Option<String>,
    plugin_api: Option<Py<PyAny>>,
    load_results: bool,
) {
    use std::{sync::{Arc, RwLock}, thread};

    use ::simusa::{api::async_api::PluginAsyncAPI, context::Context};
    use ::simusa::gui;

    use crate::general::PythonAPI;
    let async_plugin_api = plugin_api.as_ref().map(|_| Arc::new(PluginAsyncAPI::new()));
    let mut python_api = plugin_api.map(PythonAPI::new);

    let api_client = async_plugin_api.as_ref().map(|api| api.get_client());

    let running = Arc::new(RwLock::new(true));
    let local_running = running.clone();

    let context = Context::default();

    let thread_handle = thread::spawn(move || {
        while *running.read().unwrap() {
            if let Some(api_client) = &api_client {
                let python_api = python_api.as_mut().unwrap();
                // TODO: Multiple wait can be optimized the same way than AsyncAPI runner (or maybe not as it's Python)
                api_client.get_state_estimator.try_recv_closure(|request| {
                    python_api.get_state_estimator(
                        &request.config,
                        &request.global_config,
                        &request.va_factory,
                        &request.network,
                        0.,
                        &context,
                    )
                });
                api_client.get_controller.try_recv_closure(|request| {
                    python_api.get_controller(
                        &request.config,
                        &request.global_config,
                        &request.va_factory,
                        &request.network,
                        0.,
                        &context,
                    )
                });
                api_client.get_navigator.try_recv_closure(|request| {
                    python_api.get_navigator(
                        &request.config,
                        &request.global_config,
                        &request.va_factory,
                        &request.network,
                        0.,
                        &context,
                    )
                });
                api_client.get_physics.try_recv_closure(|request| {
                    python_api.get_physics(
                        &request.config,
                        &request.global_config,
                        &request.va_factory,
                        &request.network,
                        0.,
                        &context,
                    )
                });
                python_api.check_requests(&context);
            }
        }
    });

    // egui needs to be run in the main thread. However, Python get the GIL. So we need to free the GIL so that the thread spawn before can call Python functions.
    // allow_threads reacquire the GIL when it exits the function. At this point, we are sure that the other thread is finished.
    py.detach(|| {
        use ::simusa::plugin_api::PluginAPI;

        gui::run_gui(
            default_config_path,
            async_plugin_api.map(|api| api as Arc<dyn PluginAPI>),
            load_results,
        );
        *local_running.write().unwrap() = false;
        thread_handle.join().unwrap();
    });
}

#[cfg(not(feature = "gui"))]
#[pyfunction]
#[pyo3(signature = (_plugin_api=None))]
pub fn run_gui(_py: Python, _plugin_api: Option<Py<PyAny>>) {
    unimplemented!("run_gui not available. Compile python package with 'gui' feature");
}


#[pymodule]
#[pyo3(name = "simusa")]
/// Python module initializer for the `simusa` package.
pub fn simusa(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SimulatorWrapper>()?;
    m.add_class::<PluginAPIWrapper>()?;
    m.add_class::<ControllerErrorWrapper>()?;
    m.add_class::<PhysicsWrapper>()?;
    m.add_class::<StateWrapper>()?;
    m.add_class::<Pose>()?;
    m.add_class::<Vec2>()?;
    m.add_class::<Vec3>()?;
    m.add_class::<WorldStateWrapper>()?;
    m.add_class::<StateEstimatorWrapper>()?;
    m.add_class::<ObservationWrapper>()?;
    m.add_class::<SensorObservationWrapper>()?;
    m.add_class::<GNSSObservationWrapper>()?;
    m.add_class::<SpeedObservationWrapper>()?;
    m.add_class::<DisplacementObservationWrapper>()?;
    m.add_class::<OrientedLandmarkObservationWrapper>()?;
    m.add_class::<OrientedRobotObservationWrapper>()?;
    m.add_class::<ControllerWrapper>()?;
    m.add_class::<CommandWrapper>()?;
    m.add_class::<UnicycleCommandWrapper>()?;
    m.add_class::<NavigatorWrapper>()?;
    m.add_class::<NodeWrapper>()?;
    m.add_class::<MultiClientWrapper>()?;
    m.add_class::<MessageFlag>()?;
    m.add_class::<MessageTypesWrapper>()?;
    m.add_class::<GoToMessage>()?;
    m.add_class::<SensorTriggerMessage>()?;
    m.add_function(wrap_pyfunction!(run_gui, m)?)?;
    Ok(())
}
