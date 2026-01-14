use crate::{GpsRenderer, GpsVertex};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

#[wasm_bindgen]
pub struct WasmRenderer {
    renderer: Rc<RefCell<Option<GpsRenderer>>>,
    canvas: HtmlCanvasElement,
    left_dragging: bool,
    right_dragging: bool,
    last_x: f64,
    last_y: f64,
}

#[wasm_bindgen]
impl WasmRenderer {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<WasmRenderer, JsValue> {
        console_error_panic_hook::set_once();

        #[cfg(target_arch = "wasm32")]
        console_log::init_with_level(log::Level::Debug).ok();

        Ok(WasmRenderer {
            renderer: Rc::new(RefCell::new(None)),
            canvas,
            left_dragging: false,
            right_dragging: false,
            last_x: 0.0,
            last_y: 0.0,
        })
    }

    #[wasm_bindgen]
    pub async fn initialize(&mut self) -> Result<(), JsValue> {
        let _window = web_sys::window().ok_or("No window")?;
        let width = self.canvas.width();
        let height = self.canvas.height();

        log::debug!("Initializing WASM renderer with size: {}x{}", width, height);

        let renderer = GpsRenderer::new_wasm(&self.canvas)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to create renderer: {}", e)))?;

        *self.renderer.borrow_mut() = Some(renderer);

        log::debug!("WASM renderer initialized successfully");
        Ok(())
    }

    #[wasm_bindgen]
    pub fn load_activities(&mut self, vertices_json: &str) -> Result<(), JsValue> {
        let activities: Vec<Vec<[f64; 3]>> = serde_json::from_str(vertices_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse JSON: {}", e)))?;

        // todo: i should ask someone who knows pretty colors
        let activity_colors = [
            [0.8, 0.0, 0.0, 1.0],
            [0.0, 0.0, 0.8, 1.0],
            [0.0, 0.6, 0.0, 1.0],
            [0.8, 0.5, 0.0, 1.0],
            [0.6, 0.0, 0.6, 1.0],
            [0.0, 0.5, 0.5, 1.0],
            [0.8, 0.0, 0.4, 1.0],
            [0.4, 0.0, 0.8, 1.0],
        ];

        let mut all_activities = Vec::new();

        for (idx, activity) in activities.iter().enumerate() {
            let color = activity_colors[idx % activity_colors.len()];
            let vertices: Vec<GpsVertex> = activity
                .iter()
                .map(|&[lat, lon, alt]| {
                    GpsVertex::from_lat_lon_alt(lat, lon, Some(alt as f32), color)
                })
                .collect();
            all_activities.push(vertices);
        }

        let mut all_vertices_flat: Vec<GpsVertex> =
            all_activities.iter().flatten().copied().collect();
        GpsVertex::normalize_batch(&mut all_vertices_flat);

        let mut result = Vec::new();
        let mut offset = 0;
        for activity in &all_activities {
            let count = activity.len();
            result.push(all_vertices_flat[offset..offset + count].to_vec());
            offset += count;
        }

        if let Some(renderer) = self.renderer.borrow_mut().as_mut() {
            renderer.load_activities(result);

            let camera = renderer.camera_mut();
            camera.position = [-0.34, -0.64, 1.0];
            camera.up = [0.0, 0.0, 1.0];

            renderer.update_camera();
        }

        Ok(())
    }

    #[wasm_bindgen]
    pub fn resize(&mut self, width: u32, height: u32) {
        if let Some(renderer) = self.renderer.borrow_mut().as_mut() {
            renderer.resize(wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            });
        }
    }

    #[wasm_bindgen]
    pub fn render(&mut self) -> Result<(), JsValue> {
        if let Some(renderer) = self.renderer.borrow_mut().as_mut() {
            renderer.update_camera();
            renderer
                .render(false)
                .map_err(|e| JsValue::from_str(&format!("Render error: {}", e)))?;
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn on_mouse_down(&mut self, button: u16, x: f64, y: f64) {
        self.last_x = x;
        self.last_y = y;

        if button == 0 {
            self.left_dragging = true;
        } else if button == 2 {
            self.right_dragging = true;
        }
    }

    #[wasm_bindgen]
    pub fn on_mouse_up(&mut self, button: u16) {
        if button == 0 {
            self.left_dragging = false;
        } else if button == 2 {
            self.right_dragging = false;
        }
    }

    #[wasm_bindgen]
    pub fn on_mouse_move(&mut self, x: f64, y: f64) {
        if !self.left_dragging && !self.right_dragging {
            return;
        }

        let dx = (x - self.last_x) as f32;
        let dy = (y - self.last_y) as f32;

        if let Some(renderer) = self.renderer.borrow_mut().as_mut() {
            let camera = renderer.camera_mut();

            if self.left_dragging {
                let sensitivity = 0.002;
                camera.orbit(dx * sensitivity, -dy * sensitivity);
            }

            if self.right_dragging {
                let sensitivity = 0.003;
                camera.pan(dx * sensitivity, -dy * sensitivity);
            }
        }

        self.last_x = x;
        self.last_y = y;
    }

    #[wasm_bindgen]
    pub fn on_wheel(&mut self, delta_y: f64) {
        if let Some(renderer) = self.renderer.borrow_mut().as_mut() {
            let zoom_amount = (delta_y as f32) * 0.001;
            renderer.camera_mut().dolly(zoom_amount);
        }
    }

    #[wasm_bindgen]
    pub fn on_key_down(&mut self, key: &str) {
        if let Some(renderer) = self.renderer.borrow_mut().as_mut() {
            let camera = renderer.camera_mut();

            match key {
                "w" | "W" => camera.fly_forward(0.02),
                "s" | "S" => camera.fly_forward(-0.02),
                "a" | "A" => camera.fly_right(-0.02),
                "d" | "D" => camera.fly_right(0.02),
                "1" => camera.set_top_down_view(3.0),
                "2" => camera.set_perspective_view(3.0),
                "ArrowUp" => camera.pan(0.0, 0.1),
                "ArrowDown" => camera.pan(0.0, -0.1),
                "ArrowLeft" => camera.pan(-0.1, 0.0),
                "ArrowRight" => camera.pan(0.1, 0.0),
                _ => {}
            }
        }
    }
}
