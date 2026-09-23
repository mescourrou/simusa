use std::sync::{Arc, Mutex};

use eframe::egui_glow::painter;
use egui::{Rect, Response, Shape, Vec2};
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
    /// You can use [`TimeOrderedData`](simba_com::time_ordered_data::TimeOrderedData) to manage the records.
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
    #[serde(skip)]
    drawn_robots: Vec<String>,
    #[serde(skip)]
    drawables: Vec<Box<dyn Drawable>>,
    enabled_drawables: Vec<bool>,
}

impl DrawableManager {
    pub fn new() -> Self {
        Self {
            drawn_robots: Vec::new(),
            drawables: Vec::new(),
            enabled_drawables: Vec::new(),
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
        self.drawables.clear();
        self.drawables.push(Box::new(map::Map::init(
            environment.map().record(context),
            config,
            context,
        )));
        for robot in &config.robots {
            if robot.autospawn {
                self.drawables.push(Box::new(
                    robot::Robot::init(robot, config, robot.name.clone()),
                ));
                self.drawn_robots.push(robot.name.clone());
            }
        }
        if let Some(plugin_api) = plugin_api
            && let Some(drawable) = plugin_api.get_drawable(config)
        {
            self.drawables.push(drawable);
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
        for drawable in &self.drawables {
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
        for drawable in self.drawables.iter_mut() {
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
                        self.drawables.push(Box::new(
                            robot::Robot::init(new_config, config, rec.name.clone()),
                        ));
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
}