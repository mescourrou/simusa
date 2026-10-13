/*!
Module serving the [`Simulator`] with the configuration and record structures.

The [`Simulator`] is the primary struct to be called to start the simulator,
the simulator can be used as follows:
```no_run
use std::path::Path;
use simusa::simulator::Simulator;

println!("Load configuration...");
let mut simulator = Simulator::from_config_path(
    Path::new("config_example/config.yaml"),
    None, //<- plugin API, to load external modules
).unwrap();

// Show the simulator loaded configuration
simulator.show();

// Run the simulator for the time given in the configuration
// It also save the results to json
simulator.run().unwrap();

simulator.compute_results().unwrap();

```


*/

mod results;
use results::ResultSavingData;
pub use results::{ResultConfig, ResultSaveMode, Results};

mod simulator_config;
pub use simulator_config::OptimizationConfig;
pub use simulator_config::SimulatorConfig;

mod async_simulator;
use async_simulator::SimulatorAsyncApiServer;
pub use async_simulator::{AsyncSimulator, SimulatorAsyncApi};

use config_checker::ConfigCheckable;
use serde_derive::{Deserialize, Serialize};

use simusa_com::pub_sub::{PathBroker, PathMultiClient};

use crate::context::Context;
use crate::error;
use crate::info;
use crate::internal;
use crate::node::node_factory::NodeConfig;
use crate::result_analyser::ResultAnalyser;
use crate::utils::log_rotation::create_new_file_with_rotation;
use crate::warning;
use crate::{
    VERSION,
    api::internal_api::NodeClient,
    constants::TIME_ROUND,
    environment::Environment,
    errors::{SimusaError, SimusaErrorTypes, SimusaResult},
    logger::LoggerConfig,
    networking::{network::Envelope, network_manager::NetworkManager},
    node::{
        Node, NodeState,
        node_factory::{MakeNodeParams, NodeFactory, RecordType},
    },
    plugin_api::PluginAPI,
    recordable::Recordable,
    scenario::{Scenario, config::ScenarioConfig},
    time_analysis::{TimeAnalysisConfig, TimeAnalysisFactory},
    utils::{
        SharedMutex, SharedRwLock, determinist_random_variable::DeterministRandomVariableFactory,
        maths::round_precision,
    },
};
use std::{cmp::Ordering, collections::BTreeMap, fs::OpenOptions, io::SeekFrom, path::Path};

use serde_json;
use std::default::Default;
use std::fs::File;
use std::io::prelude::*;
use std::sync::{Arc, Mutex};

/// One time record.
///
/// This is a line for one record ([`RecordType`]) at a given time.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    /// Time of the record.
    pub time: f32,
    /// Record of a node.
    pub node: RecordType,
}

impl Ord for Record {
    fn cmp(&self, other: &Self) -> Ordering {
        if (self.time - other.time).abs() < TIME_ROUND {
            self.node
                .name()
                .unwrap_or(&"".to_string())
                .cmp(other.node.name().unwrap_or(&"".to_string()))
        } else {
            self.time.total_cmp(&other.time)
        }
    }
}

impl PartialOrd for Record {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Record {
    fn eq(&self, other: &Self) -> bool {
        if (self.time - other.time).abs() < TIME_ROUND {
            self.node
                .name()
                .unwrap_or(&"".to_string())
                .eq(other.node.name().unwrap_or(&"".to_string()))
        } else {
            false
        }
    }
}

impl Eq for Record {}

/// Broker type used by the simulator network.
pub type SimusaBroker = PathBroker<Envelope, String, Option<[f32; 2]>>;
/// Multi-client handle type associated with [`SimusaBroker`].
pub type SimusaBrokerMultiClient = PathMultiClient<Envelope, String>;

/// This is the central structure which manages the run of the scenario.
///
/// To run the scenario, there are two mandatory steps:
/// * Load the config using [`Simulator::from_config_path`] (from a file), or using
///   [`Simulator::from_config`] with the [`SimulatorConfig`] structs directly.
/// * Run the scenario, once the config is loaded, the scenario can be run using
///   [`Simulator::run`].
///
/// Optionnal steps are the following:
/// * Use [`Simulator::show`] to print in the console the configuration loaded.
/// * Compute the results with [`Simulator::compute_results`].
/// * Get the records with [`Simulator::get_records`], providing the full list of all [`Record`]s.
///
/// ## Example
/// ```no_run
/// use log::info;
/// use simusa::simulator::Simulator;
/// use std::path::Path;
///
/// info!("Load configuration...");
/// let mut simulator = Simulator::from_config_path(
///     Path::new("config_example/config.yaml"), //<- configuration path
///     None,                                      //<- plugin API, to load external modules
/// ).unwrap();
///
/// // Show the simulator loaded configuration
/// simulator.show();
///
/// // It also save the results to "result.json",
/// simulator.run().unwrap();
///
/// // compute the results and show the figures.
/// simulator.compute_results().unwrap();
///
/// ```
pub struct Simulator {
    /// List of the [`Node`]. Using `Arc` and `RwLock` for multithreading.
    nodes: Vec<Node>,
    /// Scenario configuration.
    config: SimulatorConfig,
    /// Network Manager
    network_manager: NetworkManager,
    /// Factory for components to make random variables generators
    determinist_va_factory: Arc<DeterministRandomVariableFactory>,
    max_threads: usize,

    async_api: Option<Arc<SimulatorAsyncApi>>,
    async_api_server: Option<SimulatorAsyncApiServer>,

    node_apis: BTreeMap<String, NodeClient>,

    result_saving_data: Option<ResultSavingData>,
    records: Vec<Record>,
    new_records: Vec<Record>,
    time_analysis_factory: Option<TimeAnalysisFactory>,
    force_send_results: bool,
    scenario: SharedMutex<Scenario>,
    plugin_api: Option<Arc<dyn PluginAPI>>,
    environment: Arc<Environment>,
    context: Context,
    last_sim_time: f32,

    result_analyser: Option<ResultAnalyser>,
}

impl Simulator {
    /// Create a new [`Simulator`] with no nodes, and empty config.
    pub fn new() -> Simulator {
        let rng = rand::random();
        let va_factory = Arc::new(DeterministRandomVariableFactory::new(rng));
        let network_manager = NetworkManager::new();
        let broker = network_manager.broker().clone();
        #[cfg(feature = "monothreaded")]
        let max_threads = 1;
        #[cfg(not(feature = "monothreaded"))]
        let max_threads = 0;
        Simulator {
            nodes: Vec::new(),
            config: SimulatorConfig::default(),
            network_manager,
            determinist_va_factory: va_factory.clone(),
            async_api: None,
            async_api_server: None,
            node_apis: BTreeMap::new(),
            result_saving_data: Some(ResultSavingData::default()),
            records: Vec::new(),
            new_records: Vec::new(),
            time_analysis_factory: Some(
                TimeAnalysisFactory::init_from_config(&TimeAnalysisConfig::default()).unwrap(),
            ),
            force_send_results: false,
            scenario: Arc::new(Mutex::new(Scenario::from_config(
                &ScenarioConfig::default(),
                &SimulatorConfig::default(),
                &va_factory,
                &broker,
            ))),
            plugin_api: None,
            environment: Arc::new(Environment::default()),
            max_threads,
            context: Context::new(&LoggerConfig::default(), Some(0.)),
            last_sim_time: -1.,
            result_analyser: None,
        }
    }

    /// Load the config from a file compatible with [`confy`], see [`SimulatorConfig`]. Initialize the [`Simulator`].
    ///
    /// ## Arguments
    /// * `config_path` - `Path` to the config file (see example in `config_example/config.yaml`).
    /// * `plugin_api`  - Provide an implementation of [`PluginAPI`] if you want to use external modules.
    ///
    /// ## Return
    /// Returns a [`Simulator`] ready to be run.
    pub fn from_config_path(
        config_path: &Path,
        plugin_api: Option<Arc<dyn PluginAPI>>,
    ) -> SimusaResult<Simulator> {
        let mut sim = Simulator::new();
        sim.load_config_path(config_path, plugin_api)?;
        Ok(sim)
    }

    /// Load the config from structure instance.
    ///
    /// ## Arguments
    /// * `config` - Scenario configuration ([`SimulatorConfig`]).
    /// * `plugin_api`  - Provide an implementation of [`PluginAPI`] if you want to use external modules.
    ///
    /// ## Return
    /// Returns a [`Simulator`] ready to be run.
    pub fn from_config(
        config: &SimulatorConfig,
        plugin_api: Option<Arc<dyn PluginAPI>>,
    ) -> SimusaResult<Simulator> {
        let mut simulator = Simulator::new();
        simulator.load_config(config, plugin_api)?;
        Ok(simulator)
    }

    /// Reset the simulator state from the currently loaded configuration.
    pub fn reset(&mut self, plugin_api: Option<Arc<dyn PluginAPI>>) -> SimusaResult<()> {
        self.context.update_time(0.);
        info!(self.context, "Reset simulator");
        self.last_sim_time = -1.;
        self.network_manager.reset();
        self.environment.clear_meta_data();
        self.nodes = Vec::new();
        self.node_apis.clear();
        let config = self.config.clone();
        self.max_threads = config.optimization.threads;
        self.result_analyser = None;

        self.time_analysis_factory = match &config.time_analysis {
            Some(time_analysis) => Some(TimeAnalysisFactory::init_from_config(time_analysis)?),
            None => None,
        };

        if config.results.is_some() && self.async_api.is_none() {
            self.async_api = Some(self.get_async_api());
        }

        self.result_saving_data = self.config.results.as_ref().map(|cfg| ResultSavingData {
            save_mode: cfg.save_mode.clone(),
            ..Default::default()
        });
        self.records.clear();
        self.new_records.clear();

        self.result_analyser = match &config.results {
            Some(result_config) => match &result_config.analyser {
                Some(analyser_config) => Some(ResultAnalyser::from_config(
                    analyser_config,
                    &self.config,
                    &self.plugin_api,
                    &self.context,
                )?),
                None => None,
            },
            None => None,
        };

        self.plugin_api = plugin_api.clone();

        self.environment = Arc::new(Environment::from_config(
            &config.environment,
            &self.determinist_va_factory,
        )?);

        // Create nodes
        for node_config in &config.nodes {
            if node_config.autospawn {
                self.add_node(
                    node_config,
                    &config,
                    self.force_send_results,
                    0.,
                    &self.context.clone(),
                )?;
            }
        }

        self.scenario = Arc::new(Mutex::new(Scenario::from_config(
            &config.scenario,
            &config,
            &self.determinist_va_factory,
            &self.network_manager.broker(),
        )));

        for node in self.nodes.iter_mut() {
            info!(self.context, "Finishing initialization of {}", node.name());
            self.node_apis.insert(
                node.name(),
                node.post_creation_init(
                    self.environment.get_meta_data().clone(),
                    0.,
                    &self.context.clone(),
                ),
            );
        }

        let map_record = Record {
            time: 0.,
            node: RecordType::Map(Box::new(self.environment.map().record(&self.context))),
        };
        if let Some(async_api_server) = &self.async_api_server {
            async_api_server.send_record(&map_record);
        }

        Ok(())
    }

    pub(crate) fn load_config_path(
        &mut self,
        config_path: &Path,
        plugin_api: Option<Arc<dyn PluginAPI>>,
    ) -> SimusaResult<()> {
        self.load_config_path_full(config_path, plugin_api, false)
    }

    pub(crate) fn load_config_path_full(
        &mut self,
        config_path: &Path,
        plugin_api: Option<Arc<dyn PluginAPI>>,
        force_send_results: bool,
    ) -> SimusaResult<()> {
        println!("Load configuration from {:?}", config_path);
        let config = SimulatorConfig::load_from_path(config_path)?;
        self.load_config_full(&config, plugin_api, force_send_results)
    }

    /// Load a simulator configuration into an existing simulator instance.
    pub fn load_config(
        &mut self,
        config: &SimulatorConfig,
        plugin_api: Option<Arc<dyn PluginAPI>>,
    ) -> SimusaResult<()> {
        self.load_config_full(config, plugin_api, false)
    }

    pub(crate) fn load_config_full(
        &mut self,
        config: &SimulatorConfig,
        plugin_api: Option<Arc<dyn PluginAPI>>,
        force_send_results: bool,
    ) -> SimusaResult<()> {
        println!("Checking configuration...");
        self.init_log(&config.log)?;
        let check_result = config.check();
        print!("{}", check_result);
        if check_result.is_ok() {
            println!("Config valid");
        } else {
            let e = SimusaError::new(
                SimusaErrorTypes::ConfigError,
                format!("Error in config: {}", check_result),
            );
            error!(self.context, "{}", e.detailed_error());
            return Err(e);
        };
        let config_version: Vec<usize> = config
            .version
            .split(".")
            .map(|s| s.parse().expect("Config version pattern not recognized"))
            .collect();
        if config_version.len() < 2 {
            return Err(SimusaError::new(
                SimusaErrorTypes::ConfigError,
                "Version is expected to be XX.YY at least".to_string(),
            ));
        }
        if config_version[0] != env!("CARGO_PKG_VERSION_MAJOR").parse::<usize>().unwrap()
            || config_version[1] != env!("CARGO_PKG_VERSION_MINOR").parse::<usize>().unwrap()
        {
            warning!(
                self.context,
                "Config major version ({}) differs from software version ({})",
                config.version,
                VERSION
            );
        }
        self.config = config.clone();
        if let Some(seed) = config.random_seed {
            self.determinist_va_factory.set_global_seed(seed);
        } else {
            self.config.random_seed = Some(self.determinist_va_factory.global_seed());
        }
        self.force_send_results = force_send_results;

        self.reset(plugin_api)
    }

    /// Return a clone of the currently loaded configuration.
    pub fn config(&self) -> SimulatorConfig {
        self.config.clone()
    }

    fn init_log(&mut self, log_config: &LoggerConfig) -> SimusaResult<()> {
        self.context.update_config(log_config);
        self.context.update_time(0.);
        Ok(())
    }

    /// Add a [`Node`] to the [`Simulator`].
    ///
    /// This function add the [`Node`] to the [`Simulator`] list and to the [`NetworkManager`].
    /// It also adds the [`NetworkManager`] to the new [`Node`].
    ///
    /// ## Arguments
    /// * `node_config` - Configuration of the [`Node`](crate::node::node_factory::NodeType::Node).
    /// * `global_config` - Full configuration of the simulation.
    /// * `force_send_results` - Whether the node should send results to the simulator even if the results are not used for the scenario or the analysis. This can be used to force the sending of results for a node, even if it is not required, for example to be able to show its state in the GUI.
    /// * `initial_time` - Initial time for the node, can be not null if the node is added during the simulation (for example in a scenario spawn event).
    fn add_node(
        &mut self,
        node_config: &NodeConfig,
        global_config: &SimulatorConfig,
        force_send_results: bool,
        initial_time: f32,
        context: &Context,
    ) -> SimusaResult<()> {
        let context: Context =
            context.new_callstack_level(&format!("add_node({})", node_config.name));
        let new_node = NodeFactory::make_node(
            node_config,
            &mut MakeNodeParams {
                plugin_api: &self.plugin_api,
                global_config,
                va_factory: &self.determinist_va_factory,
                time_analysis_factory: self.time_analysis_factory.as_mut(),
                force_send_results,
                new_name: None,
                broker: &self.network_manager.broker(),
                initial_time,
                environment: self.environment.clone(),
                context,
            },
        )?;
        let meta_data = new_node.meta_data();
        let name = meta_data.read().unwrap().name.clone();
        self.environment.insert_meta_data(name, meta_data);
        if new_node.state() != NodeState::Running {
            return Ok(());
        }
        self.nodes.push(new_node);
        Ok(())
    }

    /// Simply print the Simulator state, using stdout.
    pub fn show(&self) {
        println!("Config:");
        self.show_config();
        self.show_state(None);
    }

    /// Show the configuration loaded
    pub fn show_config(&self) {
        println!("{:#?}", self.config);
    }

    /// Show simulator state
    pub fn show_state(&self, node_name: Option<&str>) {
        if let Some(node_name) = node_name {
            if let Some(node) = self.nodes.iter().find(|n| n.name() == node_name) {
                println!("State of node '{}':\n{:#?}", node_name, node);
            } else {
                println!("Node '{}' not found", node_name);
            }
        } else {
            println!("Simulator at time {:.4}:", self.last_sim_time);
            for node in self.nodes.iter() {
                println!("- {:#?}", node);
            }
        }
    }

    /// Override the maximum simulated time for subsequent runs.
    pub fn set_max_time(&mut self, max_time: f32) {
        self.config.max_time = max_time;
    }

    /// Run the scenario until the given time.
    ///
    /// This function starts one thread by [`Node`]. It waits that the thread finishes.
    ///
    /// After the scenario is done, the results are not processed. Use [`Simulator::compute_results`] to process the results and compute the analysis.
    pub fn run(&mut self) -> SimusaResult<()> {
        let context = self.context.new_callstack_level("run");
        let max_time = self.config.max_time;

        if let Some(data) = &self.result_saving_data {
            match data.save_mode {
                ResultSaveMode::AtTheEnd => {}
                _ => self.prepare_save_results()?,
            }
        }

        let mut error = None;
        if cfg!(feature = "monothreaded") || self.max_threads == 1 {
            let mut node_contexts = self
                .nodes
                .iter()
                .map(|node| (node.name(), context.new_node_context(node.name())))
                .collect::<BTreeMap<_, _>>();
            for node in self.nodes.iter_mut() {
                if node.state() != NodeState::Running {
                    return Err(SimusaError::new(
                        SimusaErrorTypes::ImplementationError,
                        format!(
                            "Node {} not in Running state at start of run_one_node",
                            node.name()
                        ),
                    ));
                }
                info!(node_contexts[&node.name()], "Start node {}", node.name());
            }
            let mut previous_time = self.last_sim_time;
            loop {
                let mut time = f32::INFINITY;
                // if *node_sync_params.time_cv.force_finish.lock().unwrap() {
                //     break;
                // }
                for node in self
                    .nodes
                    .iter_mut()
                    .filter(|n| n.state() != NodeState::Terminated)
                {
                    match node.next_time_step(
                        previous_time + TIME_ROUND / 2.,
                        &node_contexts[&node.name()],
                    ) {
                        Ok(t) => {
                            if t < time {
                                time = t;
                                internal!(
                                    node_contexts[&node.name()],
                                    crate::logger::InternalLog::NodeSyncDetailed,
                                    "Node {} set next_time to {time}",
                                    node.name()
                                );
                            }
                        }
                        Err(e) => {
                            error = Some(e);
                            break;
                        }
                    }
                }
                if error.is_some() {
                    break;
                }
                internal!(
                    context,
                    crate::logger::InternalLog::NodeSyncDetailed,
                    "Got next_time: {time}"
                );

                if let Some(async_api_server) = &self.async_api_server {
                    async_api_server.update_time(time);
                }
                self.context.update_time(time);
                for context in node_contexts.values_mut() {
                    context.update_time(time);
                }
                if time > max_time + TIME_ROUND / 2. {
                    break;
                }

                info!(context, "Run time {}", time);
                for node in self
                    .nodes
                    .iter_mut()
                    .filter(|n| n.state() != NodeState::Terminated)
                {
                    node.process_messages();
                    node.physics_update(time, &node_contexts[&node.name()]);
                }
                let node_states = self.node_positions();
                if let Err(e) = self
                    .network_manager
                    .process_messages(&node_states, &context)
                {
                    error = Some(e);
                    break;
                }
                for node in self
                    .nodes
                    .iter_mut()
                    .filter(|n| n.state() != NodeState::Terminated)
                {
                    if node.process_messages() {
                        node.handle_messages(time, &node_contexts[&node.name()]);
                    }
                    node.pre_loop_hooks(time, &node_contexts[&node.name()]);
                }
                if let Err(e) = self
                    .network_manager
                    .process_messages(&node_states, &context)
                {
                    error = Some(e);
                    break;
                }
                let mut do_control_loops = Vec::new();
                for node in self
                    .nodes
                    .iter_mut()
                    .filter(|n| n.state() != NodeState::Terminated)
                {
                    if node.process_messages() {
                        node.handle_messages(time, &node_contexts[&node.name()]);
                    }
                    do_control_loops.push(node.prediction_step(time, &node_contexts[&node.name()]));
                }
                if let Err(e) = self
                    .network_manager
                    .process_messages(&node_states, &context)
                {
                    error = Some(e);
                    break;
                }
                for node in self
                    .nodes
                    .iter_mut()
                    .filter(|n| n.state() != NodeState::Terminated)
                {
                    if node.process_messages() {
                        node.handle_messages(time, &node_contexts[&node.name()]);
                    }
                    node.make_observations(time, &node_contexts[&node.name()]);
                }
                if let Err(e) = self
                    .network_manager
                    .process_messages(&node_states, &context)
                {
                    error = Some(e);
                    break;
                }
                for node in self
                    .nodes
                    .iter_mut()
                    .filter(|n| n.state() != NodeState::Terminated)
                {
                    if node.process_messages() {
                        node.handle_messages(time, &node_contexts[&node.name()]);
                    }
                    node.correction_step(time, &node_contexts[&node.name()]);
                }

                if let Err(e) = self
                    .network_manager
                    .process_messages(&node_states, &context)
                {
                    error = Some(e);
                    break;
                }
                for (node, do_control_loop) in self
                    .nodes
                    .iter_mut()
                    .filter(|n| n.state() != NodeState::Terminated)
                    .zip(do_control_loops)
                {
                    if node.process_messages() {
                        node.handle_messages(time, &node_contexts[&node.name()]);
                    }
                    if let Err(e) = node.nav_and_control_step(
                        time,
                        do_control_loop,
                        &node_contexts[&node.name()],
                    ) {
                        error = Some(e);
                        break;
                    }
                    if node.process_messages() {
                        node.handle_messages(time, &node_contexts[&node.name()]);
                    }
                }

                internal!(
                    context,
                    crate::logger::InternalLog::NodeSyncDetailed,
                    "End of time step"
                );

                let node_states = self.node_positions();
                if let Err(e) = self.end_of_time_step_procedure(&node_states, &context) {
                    error = Some(e);
                    break;
                }

                for node in self
                    .nodes
                    .iter_mut()
                    .filter(|n| n.state() != NodeState::Terminated)
                {
                    if node.send_records()
                        && let Some(async_api_server) = &self.async_api_server
                    {
                        async_api_server.send_record(&Record {
                            time,
                            node: node.record(&node_contexts[&node.name()]),
                        });
                    }

                    if node.process_messages() {
                        node.handle_messages(time, &node_contexts[&node.name()]);
                    }
                    if node.state() == NodeState::Zombie {
                        info!(context, "Killing node {}", node.name());
                        if node.process_messages() {
                            node.handle_messages(time, &node_contexts[&node.name()]);
                        }
                        node.kill(time);
                    }
                }
                previous_time = time;
            }
        } else {
            #[cfg(not(feature = "monothreaded"))]
            {
                use scoped_threadpool::Pool;
                if self.max_threads == 0 {
                    self.max_threads = num_cpus::get();
                }

                let mut pool = Pool::new(self.max_threads as u32);
                let mut node_contexts = self
                    .nodes
                    .iter()
                    .map(|node| (node.name(), context.new_node_context(node.name())))
                    .collect::<BTreeMap<_, _>>();
                for node in self.nodes.iter_mut() {
                    if node.state() != NodeState::Running {
                        return Err(SimusaError::new(
                            SimusaErrorTypes::ImplementationError,
                            format!(
                                "Node {} not in Running state at start of run_one_node",
                                node.name()
                            ),
                        ));
                    }
                    info!(node_contexts[&node.name()], "Start node {}", node.name());
                }
                let mut previous_time = self.last_sim_time;
                loop {
                    let mut time = f32::INFINITY;
                    // if *node_sync_params.time_cv.force_finish.lock().unwrap() {
                    //     break;
                    // }
                    for node in self
                        .nodes
                        .iter_mut()
                        .filter(|n| n.state() != NodeState::Terminated)
                    {
                        match node.next_time_step(
                            previous_time + TIME_ROUND / 2.,
                            &node_contexts[&node.name()],
                        ) {
                            Ok(t) => {
                                if t < time {
                                    time = t;
                                    internal!(
                                        node_contexts[&node.name()],
                                        crate::logger::InternalLog::NodeSyncDetailed,
                                        "Node {} set next_time to {time}",
                                        node.name()
                                    );
                                }
                            }
                            Err(e) => {
                                error = Some(e);
                                break;
                            }
                        }
                    }
                    if error.is_some() {
                        break;
                    }
                    internal!(
                        context,
                        crate::logger::InternalLog::NodeSyncDetailed,
                        "Got next_time: {time}"
                    );

                    if let Some(async_api_server) = &self.async_api_server {
                        async_api_server.update_time(time);
                    }
                    self.context.update_time(time);
                    for context in node_contexts.values_mut() {
                        context.update_time(time);
                    }
                    if time > max_time + TIME_ROUND / 2. {
                        break;
                    }

                    info!(context, "Run time {}", time);
                    pool.scoped(|scope| {
                        for node in self
                            .nodes
                            .iter_mut()
                            .filter(|n| n.state() != NodeState::Terminated)
                        {
                            let node_context = node_contexts[&node.name()].clone();
                            scope.execute(move || {
                                node.process_messages();
                                node.physics_update(time, &node_context);
                            });
                        }
                    });
                    let node_states = self.node_positions();
                    if let Err(e) = self
                        .network_manager
                        .process_messages(&node_states, &context)
                    {
                        error = Some(e);
                        break;
                    }
                    pool.scoped(|scope| {
                        for node in self
                            .nodes
                            .iter_mut()
                            .filter(|n| n.state() != NodeState::Terminated)
                        {
                            let node_context = node_contexts[&node.name()].clone();
                            scope.execute(move || {
                                if node.process_messages() {
                                    node.handle_messages(time, &node_context);
                                }
                                node.pre_loop_hooks(time, &node_context);
                            });
                        }
                    });
                    if let Err(e) = self
                        .network_manager
                        .process_messages(&node_states, &context)
                    {
                        error = Some(e);
                        break;
                    }
                    let mut do_control_loops = Vec::new();
                    pool.scoped(|scope| {
                        let (tx, rx) = std::sync::mpsc::channel();
                        for (i, node) in self
                            .nodes
                            .iter_mut()
                            .filter(|n| n.state() != NodeState::Terminated)
                            .enumerate()
                        {
                            do_control_loops.push(false);
                            let node_context = node_contexts[&node.name()].clone();
                            let tx = tx.clone();
                            scope.execute(move || {
                                if node.process_messages() {
                                    node.handle_messages(time, &node_context);
                                }
                                tx.send((i, node.prediction_step(time, &node_context)))
                                    .unwrap();
                            });
                        }
                        for (i, do_or_do_not) in rx.iter().take(do_control_loops.len()) {
                            do_control_loops[i] = do_or_do_not;
                        }
                    });
                    if let Err(e) = self
                        .network_manager
                        .process_messages(&node_states, &context)
                    {
                        error = Some(e);
                        break;
                    }
                    pool.scoped(|scope| {
                        for node in self
                            .nodes
                            .iter_mut()
                            .filter(|n| n.state() != NodeState::Terminated)
                        {
                            let node_context = node_contexts[&node.name()].clone();
                            scope.execute(move || {
                                if node.process_messages() {
                                    node.handle_messages(time, &node_context);
                                }
                                node.make_observations(time, &node_context);
                            });
                        }
                    });
                    if let Err(e) = self
                        .network_manager
                        .process_messages(&node_states, &context)
                    {
                        error = Some(e);
                        break;
                    }
                    pool.scoped(|scope| {
                        for node in self
                            .nodes
                            .iter_mut()
                            .filter(|n| n.state() != NodeState::Terminated)
                        {
                            let node_context = node_contexts[&node.name()].clone();
                            scope.execute(move || {
                                if node.process_messages() {
                                    node.handle_messages(time, &node_context);
                                }
                                node.correction_step(time, &node_context);
                            });
                        }
                    });

                    if let Err(e) = self
                        .network_manager
                        .process_messages(&node_states, &context)
                    {
                        error = Some(e);
                        break;
                    }

                    let nav_control_error = Arc::new(Mutex::new(Ok(())));
                    pool.scoped(|scope| {
                        for (node, do_control_loop) in self
                            .nodes
                            .iter_mut()
                            .filter(|n| n.state() != NodeState::Terminated)
                            .zip(do_control_loops)
                        {
                            let node_context = node_contexts[&node.name()].clone();
                            let nav_control_error_clone = nav_control_error.clone();
                            scope.execute(move || {
                                if node.process_messages() {
                                    node.handle_messages(time, &node_context);
                                }
                                if let Err(e) =
                                    node.nav_and_control_step(time, do_control_loop, &node_context)
                                {
                                    *nav_control_error_clone.lock().unwrap() = Err(e);
                                } else {
                                    if node.process_messages() {
                                        node.handle_messages(time, &node_context);
                                    }
                                }
                            });
                        }
                    });

                    if let Err(e) = nav_control_error.lock().unwrap().as_ref() {
                        error = Some(e.clone());
                        break;
                    }

                    internal!(
                        context,
                        crate::logger::InternalLog::NodeSyncDetailed,
                        "End of time step"
                    );

                    let node_states = self.node_positions();
                    if let Err(e) = self.end_of_time_step_procedure(&node_states, &context) {
                        error = Some(e);
                        break;
                    }

                    let new_contexts = self
                        .nodes
                        .iter()
                        .filter(|n| !node_contexts.contains_key(&n.name()))
                        .map(|node| (node.name(), context.new_node_context(node.name())))
                        .collect::<BTreeMap<_, _>>();
                    node_contexts.extend(new_contexts);

                    for node in self
                        .nodes
                        .iter_mut()
                        .filter(|n| n.state() != NodeState::Terminated)
                    {
                        if node.send_records()
                            && let Some(async_api_server) = &self.async_api_server
                        {
                            async_api_server.send_record(&Record {
                                time,
                                node: node.record(&node_contexts[&node.name()]),
                            });
                        }

                        if node.process_messages() {
                            node.handle_messages(time, &node_contexts[&node.name()]);
                        }
                        if node.state() == NodeState::Zombie {
                            info!(context, "Killing node {}", node.name());
                            if node.process_messages() {
                                node.handle_messages(time, &node_contexts[&node.name()]);
                            }
                            node.kill(time);
                        }
                    }
                    previous_time = time;
                }
            }
        }

        if let Some(e) = error {
            self.process_records(None, &context).map_err(|e2| {
                SimusaError::new(e2.error_type(), format!("Error while processing previous error.\nPrevious error: {}\nLast error: {}", e.detailed_error(), e2.detailed_error()))
            })?;
            return Err(e);
        }

        self.process_records(None, &context)
    }

    pub(crate) fn spawn_node_from_name(
        &mut self,
        node_name: &str,
        new_node_name: &str,
        time: f32,
        context: &Context,
    ) -> SimusaResult<()> {
        let mut node = NodeFactory::make_node_from_name(
            node_name,
            &mut MakeNodeParams {
                plugin_api: &self.plugin_api,
                global_config: &self.config,
                va_factory: &self.determinist_va_factory,
                time_analysis_factory: self.time_analysis_factory.as_mut(),
                force_send_results: self.force_send_results,
                new_name: Some(new_node_name),
                initial_time: time,
                broker: &self.network_manager.broker(),
                environment: self.environment.clone(),
                context: context
                    .new_callstack_level(&format!("spawn_node_from_name({})", node_name)),
            },
        )?;
        let meta_data = node.meta_data();
        let name = meta_data.read().unwrap().name.clone();
        self.environment.insert_meta_data(name.clone(), meta_data);

        node.set_state(NodeState::Running);

        self.node_apis.insert(
            name.clone(),
            node.post_creation_init(self.environment.get_meta_data().clone(), time, context),
        );
        self.nodes.push(node);
        Ok(())
    }

    /// Returns the list of all [`Record`]s produced by [`Simulator::run`].
    pub fn get_records(&self, sorted: bool) -> Vec<Record> {
        let mut records = self.records.clone();
        if sorted {
            records.sort();
        }
        records
    }

    fn prepare_save_results(&mut self) -> SimusaResult<()> {
        if self.config.results.is_none() {
            return Ok(());
        }
        let result_config = self.config.results.clone().unwrap();
        let filename = result_config.result_path;
        if filename.is_none() {
            return Ok(());
        }
        let filename = self.config.get_base_path().join(filename.unwrap());

        if !self.records.is_empty() {
            // Results already started, just need to remove last line
            let mut file = match OpenOptions::new().read(true).write(true).open(&filename) {
                Err(e) => {
                    return Err(SimusaError::new(
                        SimusaErrorTypes::ConfigError,
                        format!(
                            "Impossible to open result file '{}': {}",
                            filename.to_str().unwrap(),
                            e
                        ),
                    ));
                }
                Ok(f) => f,
            };
            let file_size = file.metadata().unwrap().len();

            let mut buffer = [0u8; 1];
            let mut pos = file_size.saturating_sub(1);

            while pos > 0 {
                file.seek(SeekFrom::Start(pos)).unwrap();
                file.read_exact(&mut buffer).unwrap();

                // Remove last newline
                // WARNING: assumes that the file does not end with newline, as generated by process_records
                if buffer[0] == b'\n' {
                    file.set_len(pos).unwrap();
                    return Ok(());
                }
                pos -= 1;
            }
            unreachable!("Result file should not be empty if records exist");
        }
        info!(
            self.context,
            "Saving results to {}",
            filename.to_str().unwrap_or_default()
        );
        let mut recording_file = match create_new_file_with_rotation(&filename) {
            Err(e) => {
                return Err(SimusaError::new(
                    SimusaErrorTypes::ConfigError,
                    format!(
                        "Impossible to create result file '{}': {}",
                        filename.to_str().unwrap(),
                        e
                    ),
                ));
            }
            Ok(f) => f,
        };

        recording_file.write_all(b"{\"config\": ").unwrap();
        if let Err(e) = serde_json::to_writer(&recording_file, &self.config) {
            return Err(SimusaError::new(
                SimusaErrorTypes::ImplementationError,
                format!("Error during json serialization of config: {e}"),
            ));
        }
        recording_file.write_all(b",\n\"records\": [\n").unwrap();
        Ok(())
    }

    /// Save the results to the file given during the configuration.
    ///
    /// If the configuration of the [`Simulator`] do not contain a result path, no results are saved.
    fn process_records(&mut self, time: Option<f32>, context: &Context) -> SimusaResult<()> {
        let context = context.new_callstack_level("process_records");
        if self.config.results.is_none() {
            return Ok(());
        }
        let result_saving_data = self.result_saving_data.as_ref().unwrap().clone();
        if let ResultSaveMode::AtTheEnd = result_saving_data.save_mode {
            if time.is_some() {
                return Ok(());
            }
            self.prepare_save_results()?;
        }
        let result_saving_data = self.result_saving_data.as_mut().unwrap();
        match &mut result_saving_data.save_mode {
            ResultSaveMode::Batch(remaining_size) => {
                if time.is_some() {
                    if *remaining_size <= 1 {
                        *remaining_size = match self.config.results.as_ref().unwrap().save_mode {
                            ResultSaveMode::Batch(s) => s,
                            _ => {
                                return Err(SimusaError::new(
                        SimusaErrorTypes::ImplementationError,
                                    "Incoherence between configuration and loaded configuration for Result save_mode".to_string(),
                                ));
                            }
                        }
                    } else {
                        *remaining_size -= 1;
                        return Ok(());
                    }
                }
                // If no time is given, force save (for end)
            }
            ResultSaveMode::Periodic(next_save) => {
                if let Some(time) = time {
                    if *next_save <= time {
                        *next_save = match self.config.results.as_ref().unwrap().save_mode {
                            ResultSaveMode::Periodic(t) => {
                                round_precision(*next_save + t, TIME_ROUND).unwrap()
                            }
                            _ => {
                                return Err(SimusaError::new(
                        SimusaErrorTypes::ImplementationError,
                                    "Incoherence between configuration and loaded configuration for Result save_mode".to_string(),
                                ));
                            }
                        }
                    } else {
                        return Ok(());
                    }
                }
                // If no time is given, force save (for end)
            }
            ResultSaveMode::Continuous => (),
            ResultSaveMode::AtTheEnd => (),
        }

        if time.is_none()
            && let Some(taf) = &mut self.time_analysis_factory
        {
            // Only at the end
            taf.save_results(&context);
        }

        internal!(
            context,
            crate::logger::InternalLog::NodeRunning,
            "Collecting results"
        );
        let mut new_records = std::mem::take(&mut self.new_records);
        if let Some(async_api) = &self.async_api {
            while let Ok(record) = async_api.records.lock().unwrap().try_recv() {
                new_records.push(record)
            }
        }

        let result_config = self.config.results.clone().unwrap();
        let filename = result_config.result_path;
        if let Some(filename) = filename {
            let filename = self.config.get_base_path().join(filename);

            info!(
                context,
                "Saving results to {}",
                filename.to_str().unwrap_or_default()
            );
            let mut recording_file = match File::options().append(true).open(filename.clone()) {
                Err(e) => {
                    return Err(SimusaError::new(
                        SimusaErrorTypes::ConfigError,
                        format!(
                            "Impossible to open result file '{}': {}",
                            filename.to_str().unwrap(),
                            e
                        ),
                    ));
                }
                Ok(f) => f,
            };

            for record in &new_records {
                if result_saving_data.first_row {
                    result_saving_data.first_row = false;
                } else {
                    recording_file.write_all(b",\n").unwrap();
                }
                if let Err(e) = serde_json::to_writer(&recording_file, &record) {
                    return Err(SimusaError::new(
                        SimusaErrorTypes::ImplementationError,
                        format!(
                            "Error during json serialization of record {:?}: {}",
                            record, e
                        ),
                    ));
                }
            }
            if time.is_none() {
                // Only at the end. If crashes in between, the user need to close the json array+object manually
                recording_file.write_all(b"\n]}").unwrap();
            }
        }
        self.records.extend(new_records);
        Ok(())
    }

    /// Load results from the given result `filename` or from the named specified in the loaded configuration.
    ///
    /// Returns the maximum time found in loaded records.
    pub fn load_results(&mut self, filename: Option<String>) -> SimusaResult<f32> {
        if self.config.results.is_none() {
            return Err(SimusaError::new(
                SimusaErrorTypes::ConfigError,
                "Request for loading results but no result configuration".to_string(),
            ));
        }
        let result_config = self.config.results.clone().unwrap();
        let filename = filename.or(result_config.result_path);
        if filename.is_none() {
            return Err(SimusaError::new(
                SimusaErrorTypes::ConfigError,
                "Request for loading results but no result path in configuration".to_string(),
            ));
        }
        let filename = self.config.get_base_path().join(filename.unwrap());
        let results = Self::deserialize_results_from_file(&filename, &self.context)?;

        self.records = results.records;
        let mut max_time = f32::INFINITY;
        for record in &self.records {
            max_time = max_time.max(record.time);
            self.async_api_server.as_ref().unwrap().send_record(record);
        }
        self.async_api_server
            .as_ref()
            .unwrap()
            .update_time(max_time);
        Ok(max_time)
    }

    #[deprecated(note = "Will be removed in future release. Use load_results instead")]
    /// Deprecated alias for [`Simulator::load_results`].
    pub fn load_results_full(&mut self, filename: Option<String>) -> SimusaResult<f32> {
        self.load_results(filename)
    }

    /// Deserialize persisted simulator results from a JSON file.
    ///
    /// ## Arguments
    /// * `filename` - Path to the JSON result file.
    /// * `context` - Shared simulation context used for load/deserialization logging.
    pub fn deserialize_results_from_file(
        filename: &Path,
        context: &Context,
    ) -> SimusaResult<Results> {
        info!(
            context,
            "Loading results from file `{}`",
            filename.to_str().unwrap()
        );
        let mut recording_file = match File::open(filename) {
            Ok(file) => file,
            Err(e) => {
                return Err(SimusaError::new(
                    SimusaErrorTypes::ConfigError,
                    format!(
                        "Impossible to open record file '{}': {}",
                        filename.to_str().unwrap(),
                        e
                    ),
                ));
            }
        };
        let mut content = String::new();
        recording_file
            .read_to_string(&mut content)
            .expect("Impossible to read record file");

        info!(context, "Deserialize results...");
        Ok(serde_json::from_str(&content).expect("Error during json parsing"))
    }

    fn end_of_time_step_procedure(
        &mut self,
        node_states: &BTreeMap<String, Option<[f32; 2]>>,
        context: &Context,
    ) -> SimusaResult<()> {
        let current_time = context.get_time().expect("Simulator context has no time!!");
        self.last_sim_time = current_time;

        let scenario = self.scenario.clone();
        scenario
            .lock()
            .unwrap()
            .execute_scenario(self, node_states, context)
            .unwrap();
        self.network_manager
            .process_messages(node_states, context)
            .unwrap();
        if let Err(e) = self.process_records(Some(current_time), context) {
            error!(
                context,
                "Error in processing records at time {}: {}",
                current_time,
                e.detailed_error()
            );
            return Err(e);
        }
        Ok(())
    }

    fn node_positions(&self) -> BTreeMap<String, Option<[f32; 2]>> {
        BTreeMap::from_iter(
            self.environment
                .get_meta_data()
                .read()
                .unwrap()
                .iter()
                .filter_map(|(node_name, meta_data)| {
                    let meta_data = meta_data.read().unwrap();
                    if meta_data.state != NodeState::Terminated {
                        Some((
                            node_name.clone(),
                            if let Some(physics) = meta_data.physics.as_ref() {
                                physics
                                    .read()
                                    .unwrap()
                                    .state(self.context.get_time().unwrap(), &self.context)
                                    .map(|state| [state.pose.x, state.pose.y])
                            } else {
                                None
                            },
                        ))
                    } else {
                        None
                    }
                }),
        )
    }

    /// Compute configured post-processing results from in-memory records.
    pub fn compute_results(&mut self) -> SimusaResult<()> {
        let results = self.get_records(false);
        if let Some(analyser) = &mut self.result_analyser {
            analyser.analyse_results(
                &Results {
                    records: results,
                    config: self.config.clone(),
                },
                &self.context,
            )
        } else {
            warning!(
                self.context,
                "No result analyser configured, cannot compute results"
            );
            Ok(())
        }
    }

    /// Add a new record to the simulator's records, which will be saved to the results file if configured.
    pub fn add_record(&mut self, record: Record) {
        self.new_records.push(record);
    }

    /// Create and return a new async API client for the simulator.
    pub fn get_async_api(&mut self) -> Arc<SimulatorAsyncApi> {
        if self.async_api_server.is_none() {
            self.async_api_server = Some(SimulatorAsyncApiServer::new(0.));
        }
        Arc::new(self.async_api_server.as_mut().unwrap().new_client())
    }

    /// Get the shared message broker used by the simulator network manager.
    pub fn get_broker(&self) -> SharedRwLock<SimusaBroker> {
        self.network_manager.broker()
    }

    /// Get a context for the simulator
    pub fn get_context(&self) -> Context {
        self.context.clone()
    }

    /// Get the environment of the simulator, containing the map and the meta-data of the nodes.
    pub fn get_environment(&self) -> Arc<Environment> {
        self.environment.clone()
    }

    /// Get the list of nodes of the simulator.
    ///
    /// WARNING: The list is empty while the simulator is running in multithreaded mode,
    /// as the nodes are running in their own threads and are not accessible from the main thread.
    pub fn get_nodes(&self) -> &Vec<Node> {
        &self.nodes
    }
}

impl Default for Simulator {
    fn default() -> Self {
        Self::new()
    }
}
