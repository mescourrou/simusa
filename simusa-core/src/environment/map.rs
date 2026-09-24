//! Map representation containing multiple landmarks.

#[cfg(feature = "gui")]
use std::collections::BTreeMap;
use std::{path::Path, sync::Arc};

use serde::{Deserialize, Serialize};
use simusa_macros::{config_derives, enum_variables};

#[cfg(feature = "gui")]
use crate::{gui::{UIComponent, utils::enum_combobox}, simulator::SimulatorConfig, utils::enum_tools::ToVec};
use crate::{
    config::NumberConfig,
    environment::oriented_landmark::{
        OrientedLandmark, OrientedLandmarkConfig, OrientedLandmarkRecord,
    },
    errors::{SimusaError, SimusaErrorTypes, SimusaResult},
    recordable::Recordable,
    utils::determinist_random_variable::{
        DeterministRandomVariableFactory, RandomVariableTypeConfig,
    },
};

/// Map containing multiple [`OrientedLandmark`] and possibility for random generation.
///
/// # File example
/// ```yaml
/// landmarks:
///   - id: 1
///     x: 3
///     y: 2
///     theta: 0.7854
///     width: 2.8284
///     height: 0
///   - id: 2
///     x: 5.5
///     y: 7
///     theta: 1.5708
///     width: 3
///     height: 1
/// random:
///   - number:
///       type: Num
///       value: 5
///     distributions:
///       - type: Uniform  
///         min: [0, 0, -3.1416]
///         max: [10, 10, 3.1416]
///     variable_order: [x, y, theta]
/// ```
#[config_derives]
#[derive(Default)]
pub struct MapConfig {
    /// Landmarks contained in the map.
    pub landmarks: Vec<OrientedLandmarkConfig>,
    /// Random generation parameters.
    pub random: Vec<RandomMap>,
}

#[cfg(feature = "gui")]
impl UIComponent for MapConfig {
    fn show_mut(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        buffer_stack: &mut BTreeMap<String, String>,
        global_config: &SimulatorConfig,
        current_node_name: Option<&String>,
        unique_id: &str,
    ) {
        ui.vertical(|ui| {
            ui.label("Landmarks:");
            for (i, landmark) in self.landmarks.iter_mut().enumerate() {
                egui::CollapsingHeader::new(format!("Landmark {}", i)).show(ui, |ui| {
                    landmark.show_mut(
                        ui,
                        ctx,
                        buffer_stack,
                        global_config,
                        current_node_name,
                        format!("{}-landmark-{}", unique_id, i).as_str(),
                    );
                });
            }
            if ui.button("+ Add Landmark").clicked() {
                self.landmarks.push(OrientedLandmarkConfig::default());
            }
            ui.separator();
            ui.label("Random generation:");
            for (i, random_map) in self.random.iter_mut().enumerate() {
                egui::CollapsingHeader::new(format!("Random Map {}", i)).show(ui, |ui| {
                    random_map.show_mut(
                        ui,
                        ctx,
                        buffer_stack,
                        global_config,
                        current_node_name,
                        format!("{}-random-map-{}", unique_id, i).as_str(),
                    );
                });
            }
            if ui.button("+ Add Random Map").clicked() {
                self.random.push(RandomMap::default());
            }
        });
    }

    fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str) {
        ui.vertical(|ui| {
            ui.label("Landmarks:");
            for (i, landmark) in self.landmarks.iter().enumerate() {
                egui::CollapsingHeader::new(format!("Landmark {}", i)).show(ui, |ui| {
                    landmark.show(
                        ui,
                        ctx,
                        format!("{}-landmark-{}", unique_id, i).as_str(),
                    );
                });
            }
            ui.separator();
            ui.label("Random generation:");
            for (i, random_map) in self.random.iter().enumerate() {
                egui::CollapsingHeader::new(format!("Random Map {}", i)).show(ui, |ui| {
                    random_map.show(
                        ui,
                        ctx,
                        format!("{}-random-map-{}", unique_id, i).as_str(),
                    );
                });
            }
        });
    }
}

enum_variables!(
    "Variables of the [`RandomMap`]."
    MapVariables;
    "X position of the landmark"
    X, "x";
    "Y position of the landmark"
    Y, "y";
    "Orientation of the landmark"
    Theta, "theta", "orientation";
    "Height of the landmark"
    Height, "height", "h";
    "Width of the landmark"
    Width, "width", "w";
);

/// Configuration for generating random landmarks in the map.
#[config_derives]
#[derive(Default)]
pub struct RandomMap {
    /// Number of landmarks to generate randomly.
    pub number: NumberConfig,
    /// List of distributions to generate the landmarks. The order should be consistent
    /// with `variable_order`.
    #[check]
    pub distributions: Vec<RandomVariableTypeConfig>,
    /// Ordered list of target variables receiving sampled perturbations.
    pub variable_order: Vec<MapVariables>,
}

impl Check for RandomMap {
    fn do_check(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if !self.variable_order.is_empty()
            && self.distributions.iter().map(|d| d.dim()).sum::<usize>()
                != self.variable_order.len()
        {
            errors.push(format!("If variable order is given, its length should match the total distribution dimension. Got total distribution dimension {} and variable order length {}.",
                self.distributions.iter().map(|d| d.dim()).sum::<usize>(),
                self.variable_order.len()
            ));
        }
        if let NumberConfig::Num(n) = self.number
            && n < 0.
        {
            errors.push(format!(
                "Number of random landmarks should be non-negative, got {}",
                n
            ));
        } else if let NumberConfig::Rand(cfg) = &self.number
            && cfg.dim() != 1
        {
            errors.push(format!("Number of random landmarks should be a univariate random variable, got dimension {}", cfg.dim()));
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(feature = "gui")]
impl UIComponent for RandomMap {
    fn show_mut(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        buffer_stack: &mut BTreeMap<String, String>,
        global_config: &SimulatorConfig,
        current_node_name: Option<&String>,
        unique_id: &str,
    ) {
        ui.vertical(|ui|{
            ui.horizontal(|ui| {
                ui.label("Number of landmarks:");
                self.number.show_mut(ui, ctx, buffer_stack, global_config, current_node_name, unique_id);
            });
            ui.label("Distributions:");
            RandomVariableTypeConfig::show_vector_mut(
                &mut self.distributions,
                ui,
                ctx,
                buffer_stack,
                global_config,
                current_node_name,
                unique_id,
            );
            let possible_variables: Vec<MapVariables> = MapVariables::to_vec();
            ui.horizontal(|ui| {
                ui.label("Variable order:");
                for (i, var) in self.variable_order.iter_mut().enumerate() {
                    let unique_var_id = format!("variable-{i}-{unique_id}");
                    enum_combobox(ui, var, unique_var_id);
                }
                if !self.variable_order.is_empty() && ui.button("-").clicked() {
                    self.variable_order.pop();
                }
                if ui.button("+").clicked() {
                    self.variable_order.push(
                        possible_variables
                            .get(self.variable_order.len().min(possible_variables.len()))
                            .unwrap()
                            .clone(),
                    );
                }
            });
        });
    }

    fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str) {
        ui.vertical(|ui|{
            ui.horizontal(|ui| {
                ui.label("Number of landmarks:");
                self.number.show(ui, ctx, unique_id);
            });
            ui.label("Distributions:");
            RandomVariableTypeConfig::show_vector(
                &self.distributions,
                ui,
                ctx,
                unique_id,
            );
            ui.horizontal(|ui| {
                ui.label("Variable order:");
                for (i, var) in self.variable_order.iter().enumerate() {
                    ui.label(format!("{}: {}", i, var));
                }
            });
        });
    }
}

impl RandomMap {
    /// Generate a list of [`OrientedLandmark`] based on the configuration of this random map.
    ///
    /// # Arguments
    /// * `va_factory`: Factory for creating random variable instances based on the distribution configurations.
    /// * `first_id`: Starting ID for the generated landmarks. Each landmark will be assigned a unique ID incremented from this starting point.
    ///
    pub fn generate_landmarks(
        &self,
        va_factory: &Arc<DeterministRandomVariableFactory>,
        first_id: usize,
    ) -> Vec<OrientedLandmark> {
        let nb = match &self.number {
            NumberConfig::Num(n) => n.round() as usize,
            NumberConfig::Rand(cfg) => {
                let va = va_factory.make_variable(cfg.clone());
                va.generate(0.).first().unwrap().max(0.).round() as usize
            }
        };
        let mut landmarks = Vec::new();
        for i in 0..nb {
            let mut landmark = OrientedLandmark::from_config(&OrientedLandmarkConfig::default());
            landmark.id = (first_id + i) as i32;
            let mut random_sample = Vec::new();
            for d_config in &self.distributions {
                let va = va_factory.make_variable(d_config.clone());
                random_sample.extend_from_slice(&va.generate(0.));
            }

            for (i, var) in self.variable_order.iter().enumerate() {
                match var {
                    MapVariables::X => landmark.pose.x = random_sample[i],
                    MapVariables::Y => landmark.pose.y = random_sample[i],
                    MapVariables::Theta => landmark.pose.z = random_sample[i],
                    MapVariables::Height => landmark.height = random_sample[i],
                    MapVariables::Width => landmark.width = random_sample[i],
                }
            }
            landmarks.push(landmark);
        }

        landmarks
    }
}

/// Map representation containing multiple landmarks.
#[derive(Debug, Clone, Default)]
pub struct Map {
    /// Landmarks contained in the map.
    landmarks: Vec<OrientedLandmark>,
}

impl Map {
    /// Creates an empty map.
    pub fn new() -> Self {
        Self {
            landmarks: Vec::new(),
        }
    }

    /// Load the map from the given `path`.
    pub fn from_config(
        config: &MapConfig,
        va_factory: &Arc<DeterministRandomVariableFactory>,
    ) -> SimusaResult<Map> {
        let mut landmarks = config
            .landmarks
            .iter()
            .map(OrientedLandmark::from_config)
            .collect::<Vec<_>>();
        let mut max_id = landmarks.iter().map(|l| l.id).max().unwrap_or(0) as usize + 1;
        for random_config in &config.random {
            let new_landmarks = random_config.generate_landmarks(va_factory, max_id);
            max_id += new_landmarks.len();
            landmarks.extend(new_landmarks);
        }
        Ok(Map { landmarks })
    }

    /// Get the landmarks contained in the map.
    pub fn landmarks(&self) -> &Vec<OrientedLandmark> {
        &self.landmarks
    }
}

impl Recordable<MapRecord> for Map {
    fn record(&self, context: &crate::context::Context) -> MapRecord {
        MapRecord {
            landmarks: self
                .landmarks
                .iter()
                .map(|l| l.record(context))
                .collect::<Vec<_>>(),
        }
    }
}

/// Record type for [`Map`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapRecord {
    /// Landmarks contained in the map.
    pub landmarks: Vec<OrientedLandmarkRecord>,
}

#[cfg(feature = "gui")]
impl UIComponent for MapRecord {
    fn show(&self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str) {
        egui::CollapsingHeader::new("Landmarks:").show(ui, |ui| {
            ui.vertical(|ui| {
                for l in &self.landmarks {
                    l.show(ui, ctx, format!("{}-{}", unique_id, l.id).as_str());
                }
            })
        });
    }
}
