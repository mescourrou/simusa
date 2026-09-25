use egui::{Color32, Rect, Shape, Stroke, Vec2};
use simusa_com::time_ordered_data::TimeOrderedData;

use crate::{
    constants::TIME_ROUND,
    context::Context,
    environment::map::MapRecord,
    gui::{Drawable, app::PainterInfo},
    simulator::SimulatorConfig,
};

pub struct Map {
    color: Color32,
    maps: TimeOrderedData<MapRecord>,
    arrow_len: f32,
}

impl Default for Map {
    fn default() -> Self {
        let maps = TimeOrderedData::new(TIME_ROUND);
        Self {
            color: Color32::RED,
            maps,
            arrow_len: 0.2,
        }
    }
}

impl Map {
    /// Initialize the map drawable from environment and simulator configuration.
    ///
    /// ## Arguments
    /// * `initial_map` - Optional initial map record to populate the drawable before the simulation starts.
    ///   If `None`, the map will be empty until a record is added.
    /// * `sim_config` - Simulator configuration used to resolve the map path.
    /// * `context` - Shared simulation context used for initialization logs.
    pub fn init(
        initial_map: Option<MapRecord>,
        _sim_config: &SimulatorConfig,
        _context: &Context,
    ) -> Self {
        let mut maps = TimeOrderedData::new(TIME_ROUND);
        if let Some(map) = initial_map {
            maps.insert(0., map, true);
        }
        Self {
            color: Color32::RED,
            arrow_len: 0.2,
            maps,
        }
    }
}

impl Drawable for Map {
    fn add_map_record(&mut self, time: f32, record: MapRecord) {
        self.maps.insert(time, record, true);
    }

    /// Build the set of map shapes for rendering in the current viewport.
    fn draw(
        &self,
        _ui: &mut egui::Ui,
        _viewport: &Rect,
        painter_info: &PainterInfo,
        scale: f32,
        time: f32,
    ) -> Result<Vec<Shape>, Vec2> {
        let mut shapes = Vec::new();
        let center = painter_info.zero(scale);
        let map = match self.maps.get_data_beq_time(time) {
            Some(m) => m.1,
            None => return Ok(shapes),
        };
        for landmark in map.landmarks.iter() {
            let position = Vec2::new(landmark.x, landmark.y);
            if !painter_info.is_inside(&position) {
                return Err(position);
            }
            let arrow_tip = position
                + Vec2 {
                    x: self.arrow_len * landmark.theta.cos(),
                    y: self.arrow_len * landmark.theta.sin(),
                };
            if !painter_info.is_inside(&arrow_tip) {
                return Err(arrow_tip);
            }
            let position = center + position * scale;
            let arrow_tip = center + arrow_tip * scale;

            shapes.push(Shape::rect_filled(
                Rect::from_center_size(
                    position,
                    Vec2 {
                        x: 0.05 * scale,
                        y: 0.05 * scale,
                    },
                ),
                0.01 * scale,
                self.color,
            ));
            shapes.push(Shape::line_segment(
                [position, arrow_tip],
                Stroke {
                    color: self.color,
                    width: 0.05 * scale,
                },
            ));
            if landmark.width > 0.0 {
                let half_width = landmark.width / 2.0;
                let dir_vector = Vec2::new(
                    half_width * (landmark.theta + std::f32::consts::FRAC_PI_2).cos(),
                    half_width * (landmark.theta + std::f32::consts::FRAC_PI_2).sin(),
                );
                let p1 = position + dir_vector * scale;
                let p2 = position - dir_vector * scale;
                shapes.push(Shape::line_segment(
                    [p1, p2],
                    Stroke {
                        color: self.color,
                        width: 0.01 * scale,
                    },
                ));
                shapes.push(Shape::circle_filled(p1, 0.05 * scale, self.color));
                shapes.push(Shape::circle_filled(p2, 0.05 * scale, self.color));
            }
        }
        Ok(shapes)
    }

    fn name(&self) -> Option<String> {
        Some("Map".to_string())
    }

    fn show_params_mut(&mut self, ui: &mut egui::Ui, _unique_id: &str) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label("Color:");
                ui.color_edit_button_srgba(&mut self.color);
            });
            ui.add(egui::Slider::new(&mut self.arrow_len, 0.0..=1.0).text("Arrow Length"));
        });
    }
}
