use egui::{Color32, Rect, Response, Shape, Stroke, Vec2};
use nalgebra::Vector3;
use simusa_com::time_ordered_data::TimeOrderedData;

use crate::{
    constants::TIME_ROUND, gui::{Drawable, UIComponent, app::PainterInfo, drawables}, node::node_factory::{RecordType, RobotConfig, RobotRecord}, sensors::{SensorConfig, SensorObservationRecord}, simulator::SimulatorConfig,
};

use super::observations::{OrientedLandmarkObservation, OrientedRobotObservation};

struct Toggles {
    show_ground_truth: bool,
    show_observations: bool,
}

pub struct Robot {
    gt_color: Color32,
    records: TimeOrderedData<RobotRecord>,
    arrow_len: f32,
    landmark_obs: Option<OrientedLandmarkObservation>,
    robot_obs: Option<OrientedRobotObservation>,
    gnss_obs: Option<drawables::observations::GNSSObservation>,
    scan_obs: Option<drawables::observations::ScanObservation>,
    context_info_enabled: bool,
    robot_name: String,
    toggles: Toggles,
}

impl Robot {
    pub fn init(config: &RobotConfig, sim_config: &SimulatorConfig, robot_name: String) -> Self {
        let mut landmark_obs = None;
        let mut robot_obs = None;
        let mut gnss_obs = None;
        let mut scan_obs = None;
        for sensor_conf in &config.sensor_manager.sensors {
            match &sensor_conf.config {
                SensorConfig::GNSS(c) => {
                    gnss_obs = Some(drawables::observations::GNSSObservation::init(
                        c, sim_config,
                    ))
                }
                SensorConfig::Speed(_) => {}
                SensorConfig::Displacement(_) => {}
                SensorConfig::OrientedLandmark(c) => {
                    landmark_obs = Some(OrientedLandmarkObservation::init(c, sim_config))
                }
                SensorConfig::Robot(c) => {
                    robot_obs = Some(OrientedRobotObservation::init(c, sim_config))
                }
                SensorConfig::Scan(c) => {
                    scan_obs = Some(drawables::observations::ScanObservation::init(
                        c, sim_config,
                    ))
                }
                SensorConfig::External(_) => {}
            }
        }

        Self {
            gt_color: Color32::BLUE,
            records: TimeOrderedData::new(TIME_ROUND),
            arrow_len: 0.2,
            landmark_obs,
            robot_obs,
            gnss_obs,
            scan_obs,
            context_info_enabled: false,
            robot_name,
            toggles: Toggles {
                show_ground_truth: true,
                show_observations: true,
            },
        }
    }
}

impl Drawable for Robot {
    fn add_robot_record(&mut self, time: f32, record: RobotRecord) {
        if record.name != self.robot_name {
            return;
        }
        self.records.insert(time, record, true);
    }

    fn draw(
        &self,
        ui: &mut egui::Ui,
        viewport: &Rect,
        painter_info: &PainterInfo,
        scale: f32,
        time: f32,
    ) -> Result<Vec<Shape>, Vec2> {
        let mut shapes = Vec::new();
        let center = painter_info.zero(scale);

        if let Some((max_time, _)) = self.records.max_time()
            && time > max_time + TIME_ROUND
        {
            return Ok(shapes);
        }
        if let Some((_, record)) = self.records.get_data_beq_time(time) {
            let pose = record.physics.pose();
            if self.toggles.show_ground_truth {
                let position = Vec2::new(pose[0], pose[1]);
                if !painter_info.is_inside(&position) {
                    return Err(position);
                }
    
                let position = position * scale;
    
                let position = center + position;
                let arrow_tip = position
                    + Vec2 {
                        x: self.arrow_len * pose[2].cos() * scale,
                        y: self.arrow_len * pose[2].sin() * scale,
                    };
                shapes.push(Shape::circle_filled(position, 0.1 * scale, self.gt_color));
                shapes.push(Shape::line_segment(
                    [position, arrow_tip],
                    Stroke {
                        color: self.gt_color,
                        width: 0.05 * scale,
                    },
                ));
            }

            if self.toggles.show_observations {
                for obs in &record.sensors.last_observations {
                    match &obs.sensor_observation {
                        SensorObservationRecord::OrientedLandmark(o) => {
                            shapes.extend(self.landmark_obs.as_ref().unwrap().draw(
                                ui,
                                viewport,
                                painter_info,
                                scale,
                                o,
                                &Vector3::from(pose),
                            )?)
                        }
                        SensorObservationRecord::OrientedRobot(o) => {
                            shapes.extend(self.robot_obs.as_ref().unwrap().draw(
                                ui,
                                viewport,
                                painter_info,
                                scale,
                                o,
                                &Vector3::from(pose),
                            )?)
                        }
                        SensorObservationRecord::GNSS(o) => {
                            shapes.extend(self.gnss_obs.as_ref().unwrap().draw(
                                ui,
                                viewport,
                                painter_info,
                                scale,
                                o,
                            )?)
                        }
                        SensorObservationRecord::Scan(o) => {
                            shapes.extend(self.scan_obs.as_ref().unwrap().draw(
                                ui,
                                viewport,
                                painter_info,
                                scale,
                                o,
                                &Vector3::from(pose),
                            )?)
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(shapes)
    }

    fn react(
        &mut self,
        _ui: &mut egui::Ui,
        ctx: &egui::Context,
        response: &Response,
        painter_info: &PainterInfo,
        scale: f32,
        time: f32,
    ) {
        if let Some((t, record)) = self.records.get_data_beq_time(time) {
            let pose = record.physics.pose();
            let position = Vec2::new(pose[0], pose[1]);

            if painter_info.is_position_clicked(response.interact_pointer_pos(), scale, position) {
                self.context_info_enabled = true;
            }
            if self.context_info_enabled {
                egui::Window::new(&record.name).show(ctx, |ui| {
                    if ui.button("Close").clicked() {
                        self.context_info_enabled = false;
                    }
                    let unique_id = format!("record-robot-{}", record.name);
                    ui.label(format!("Time: {:.3} s", t));

                    egui::ScrollArea::both().show(ui, |ui| {
                        record.show(ui, ctx, &unique_id);
                    });
                });
            }
        }
    }

    fn associated_node_name(&self) -> Option<String> {
        Some(self.robot_name.clone())
    }

    fn name(&self) -> Option<String> {
        None
    }

    fn show_params_mut(&mut self, ui: &mut egui::Ui, _unique_id: &str) {
        ui.vertical(|ui| {
            ui.checkbox(&mut self.toggles.show_ground_truth, "Show Ground Truth");
            if self.toggles.show_ground_truth {
                ui.horizontal(|ui| {
                    ui.label("Ground Truth Color:");
                    ui.color_edit_button_srgba(&mut self.gt_color);
                });
                ui.add(egui::Slider::new(&mut self.arrow_len, 0.0..=1.0).text("Arrow Length"));
            }
            ui.checkbox(&mut self.toggles.show_observations, "Show Observations");
            if self.toggles.show_observations {
                if let Some(obs) = &mut self.landmark_obs {
                    ui.label("Landmark Observations:");
                    obs.show_params_mut(ui);
                }
                if let Some(obs) = &mut self.robot_obs {
                    ui.label("Robot Observations:");
                    obs.show_params_mut(ui);
                }
                if let Some(obs) = &mut self.gnss_obs {
                    ui.label("GNSS Observations:");
                    obs.show_params_mut(ui);
                }
                if let Some(obs) = &mut self.scan_obs {
                    ui.label("Scan Observations:");
                    obs.show_params_mut(ui);
                }
            }
        });
    }
}
