use std::{cell::Cell, collections::{BTreeMap, HashMap}, default, sync::{Arc, Mutex}};

use colored::Colorize;
use eframe::egui_glow::painter;
use egui::{FontId, Rect, Response, RichText, Shape, Vec2};
use serde::{Deserialize, Serialize};

use crate::{api::async_api::AsyncApiRunner, context::Context, environment::map::MapRecord, error, gui::app::PainterInfo, node::node_factory::{ComputationUnitConfig, ComputationUnitRecord, RecordType, RobotConfig, RobotRecord}, plugin_api::PluginAPI, recordable::Recordable, scenario::config::EventRecord, simulator::SimulatorConfig};

pub mod map;
pub mod observations;
pub mod popup;
pub mod robot;

/// Trait for a drawable element in the GUI. It is used to draw the elements of the simulation in the GUI, such as the robot, the landmarks, the trajectory, etc., to react to the user interaction, and to draw additionnal windows.
pub trait Drawable {
    /// Function called when a new robot record is received. The time of the record can be different from the time of the last drawing.
    ///
    /// You can use [`TimeOrderedData`](simusa_com::time_ordered_data::TimeOrderedData) to manage the records.
    ///
    /// # Arguments
    /// * `time` - The time of the record. It can be different from the time of the last drawing, and it can be in the past or in the future.
    /// * `record` - The record to add. It contains the information to draw the drawable for the given time.
    ///
    #[allow(unused_variables)]
    fn add_robot_record(&mut self, time: f32, record: RobotRecord) {}

    /// Function called when a new computation unit record is received. The time of the record can be different from the time of the last drawing.
    /// 
    /// See [`add_robot_record`](Drawable::add_robot_record) for more information.
    #[allow(unused_variables)]
    fn add_computation_unit_record(&mut self, time: f32, record: ComputationUnitRecord) {}

    /// Function called when a new map record is received. The time of the record can be different from the time of the last drawing.
    ///
    /// See [`add_robot_record`](Drawable::add_robot_record) for more information.
    #[allow(unused_variables)]
    fn add_map_record(&mut self, time: f32, record: MapRecord) {}

    /// Function called when a new scenario event record is received. The time of the record can be different from the time of the last drawing.
    /// 
    /// See [`add_robot_record`](Drawable::add_robot_record) for more information.
    #[allow(unused_variables)]
    fn add_scenario_record(&mut self, time: f32, record: EventRecord) {}

    /// Returns the name of the node associated with this drawable, to group Drawable parameters by node in the GUI.
    /// If the drawable is not associated with a node, it should return None and will be displayed in the global parameters section of the GUI.
    fn associated_node_name(&self) -> Option<String> {
        None
    }

    /// Returns the name of the drawable, to display its settings on the GUI and its toggle button.
    /// 
    /// If the drawable is not associated with a node, it should return a name.
    /// 
    /// If the drawable is associated with a node, if it returns None, the drawable will be toggled with the node global button.
    fn name(&self) -> Option<String>;

    /// Show parameters of the drawable to be customized in the GUI.
    #[allow(unused_variables)]
    fn show_params_mut(&mut self, ui: &mut egui::Ui, unique_id: &str) {}


    /// Draws the drawable for the given time.
    ///
    /// # Arguments
    /// * `ui` - The egui UI to draw the drawable on.
    /// * `viewport` - The viewport to draw the drawable in. It is used to determine if the drawable is visible or not, and to get the position of the drawable in the viewport.
    /// * `painter_info` - The painter info to draw the drawable
    /// * `scale` - The scale to draw the drawable. It is used to scale the drawable according to the zoom level of the viewport.
    /// * `time` - The time to draw the drawable for. It is used to get the record of the drawable for the given time, and to draw the drawable according to the record. If there is no record for the given time, the drawable should be drawn according to the closest record in a given limit, or not drawn at all if there is no record at all.
    ///
    /// ## Returns
    /// * `Ok(Vec<Shape>)` if the drawable could be drawn for the given time.
    /// * `Err(Vec2)` if the drawable could not be drawn for the given time because it is outside of the viewport. The `Vec2` is the position of the drawable that is outside of the viewport. This will trigger an extension of the viewport in the GUI, allowing you to see the drawable.
    fn draw(
        &self,
        ui: &mut egui::Ui,
        viewport: &Rect,
        painter_info: &PainterInfo,
        scale: f32,
        time: f32,
    ) -> Result<Vec<Shape>, Vec2>;

    /// Reacts to the user interaction with the drawable. This can be used to implement clickable drawables.
    ///
    /// The default implementation does nothing, but it can be overridden to implement custom behavior.
    ///
    /// # Arguments
    /// * `ui` - The egui UI to react to the user interaction.
    /// * `ctx` - The egui context to react to the user interaction.
    /// * `response` - The response of the user interaction with the drawable. It contains information about the type of interaction (click, hover, etc.) and the position of the interaction.
    /// * `painter_info` - The painter info to react to the user interaction such as the position of the click in the viewport.
    /// * `scale` - The scale to react to the user interaction. It is used to scale the position of the interaction according to the zoom level of the viewport.
    /// * `time` - The time to react to the user interaction for.
    ///
    #[allow(unused_variables)]
    fn react(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        response: &Response,
        painter_info: &PainterInfo,
        scale: f32,
        time: f32,
    ) {
    }
}

#[derive(Deserialize, Serialize)]
pub struct DrawableManager {
    enabled: Vec<bool>,
    #[serde(skip)]
    drawn_robots: Vec<String>,
    #[serde(skip)]
    drawables: Vec<Box<dyn Drawable>>,
    #[serde(skip)]
    categories: Cell<BTreeMap<String, Vec<usize>>>,
}

impl DrawableManager {
    const GLOBAL_CATEGORY: &str = "__GLOBAL__";
    pub fn new() -> Self {
        Self {
            drawn_robots: Vec::new(),
            drawables: Vec::new(),
            enabled: Vec::new(),
            categories: Cell::new(BTreeMap::new()),
        }
    }

    fn add_drawable(&mut self, drawable: Box<dyn Drawable>, context: &Context) {
        if let Some(category) = drawable.associated_node_name() {
            self.categories.get_mut().entry(category).or_insert_with(Vec::new).push(self.drawables.len());
        } else {
            if drawable.name().is_none() {
                error!(context, "Drawables without name and without associated node are ignored.", );
                return;
            }
            self.categories.get_mut().entry(Self::GLOBAL_CATEGORY.to_string()).and_modify(|v| v.push(self.drawables.len()));
        }
        self.drawables.push(drawable);
        if self.enabled.len() < self.drawables.len() {
            self.enabled.resize(self.drawables.len(), true);
        }
    }

    pub fn init_drawables(&mut self, config: &SimulatorConfig, plugin_api: &Option<Arc<dyn PluginAPI>>, server: &Option<Arc<Mutex<AsyncApiRunner>>>, context: &Context) {
        let environment = server
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .get_simulator()
            .lock()
            .unwrap()
            .get_environment();
        // Prepare drawable management
        self.drawables.clear();
        self.categories.set(BTreeMap::new());
        self.categories.get_mut().insert(Self::GLOBAL_CATEGORY.to_string(), Vec::new());
        // Insert Map
        self.add_drawable(Box::new(map::Map::init(
            environment.map().record(context),
            config,
            context,
        )), context);
        
        // Insert robots
        for robot in &config.robots {
            if robot.autospawn {
                self.add_drawable(Box::new(robot::Robot::init(robot, config, robot.name.clone())), context);
                self.drawn_robots.push(robot.name.clone());
            }
        }
        if let Some(plugin_api) = plugin_api {
            for drawable in plugin_api.get_drawables(config) {
                self.add_drawable(drawable, context);
            }
        }
    }

    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        viewport: Rect,
        painter_info: &PainterInfo,
        drawing_scale: f32,
        current_draw_time: f32,
    ) -> Result<Vec<Shape>, Vec2> {
        let mut shapes = Vec::new();
        for (drawable, enabled) in self.drawables.iter().zip(&self.enabled) {
            if !enabled {
                continue;
            }
            shapes.extend(drawable.draw(
                ui,
                &viewport,
                painter_info,
                drawing_scale,
                current_draw_time,
            )?);
        }
        Ok(shapes)
    }

    pub fn react(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        _viewport: Rect,
        response: &Response,
        painter_info: &PainterInfo,
        drawing_scale: f32,
        current_draw_time: f32,
    ) {
        for (drawable, enabled) in self.drawables.iter_mut().zip(&self.enabled) {
            if !enabled {
                continue;
            }
            drawable.react(
                ui,
                ctx,
                response,
                painter_info,
                drawing_scale,
                current_draw_time,
            );
        }
    }

    pub fn add_record(&mut self, time: f32, record: &RecordType, simulator_config: &Option<SimulatorConfig>, context: &Context) {
        match &record {
            RecordType::ComputationUnit(rec) => {
                for drawable in self.drawables.iter_mut() {
                    drawable.add_computation_unit_record(time, *rec.clone());
                }
            }
            RecordType::Robot(rec) => {
                if !self.drawn_robots.contains(&rec.name) && let Some(config) = simulator_config {
                    if let Some(new_config) =
                            config.robots.iter().find(|rc| rc.name == rec.model_name)
                    {
                        self.add_drawable(Box::new(robot::Robot::init(new_config, config, rec.name.clone())), context);
                        self.drawn_robots.push(rec.name.clone());
                    } else {
                        error!(
                            context,
                            "Received record for unknown robot {}", rec.name
                        );
                    }
                }
                for drawable in self.drawables.iter_mut() {
                    drawable.add_robot_record(time, *rec.clone());
                }
            }
            RecordType::Map(rec) => {
                for drawable in self.drawables.iter_mut() {
                    drawable.add_map_record(time, *rec.clone());
                }
            }
            RecordType::Scenario(rec) => {
                for drawable in self.drawables.iter_mut() {
                    drawable.add_scenario_record(time, *rec.clone());
                }
            }
        }
    }

    fn draw_one_drawable(&mut self, ui: &mut egui::Ui, index: usize, unique_id: &str) {
        if let Some(name) = self.drawables[index].name() {
            egui::CollapsingHeader::new(name)
            .id_salt(format!("{}", unique_id))
            .show(ui, |ui| {
                ui.checkbox(&mut self.enabled[index], "Enabled");
                self.drawables[index].show_params_mut(ui, unique_id);
            });
        } else {
            self.drawables[index].show_params_mut(ui, unique_id);
        }
    }

    pub fn draw_parameters(&mut self, ui: &mut egui::Ui, unique_id: &str) {
        let categories = self.categories.take();
        for (category, indices) in categories.iter() {
            let unique_id = format!("drawable-params-{}-{}", unique_id, category);
            if category != Self::GLOBAL_CATEGORY {
                ui.separator();
                egui::CollapsingHeader::new(category)
                    .id_salt(format!("{}", unique_id))
                    .show(ui, |ui| {
                        for index in indices {
                            let unique_id = format!("{}-{}", unique_id, index);
                            self.draw_one_drawable(ui, *index, &unique_id);
                        }
                    });
            } else {
                ui.label(RichText::new("Global:").strong().heading());
                for index in indices {
                    let unique_id = format!("{}-{}", unique_id, index);
                    self.draw_one_drawable(ui, *index, &unique_id);
                }
                ui.label(RichText::new("Nodes:").strong().heading());
            }
        }
        self.categories.set(categories);
    }
}