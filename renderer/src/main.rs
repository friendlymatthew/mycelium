#[cfg(feature = "desktop")]
use renderer::{GpsRenderer, GpsVertex};
#[cfg(feature = "desktop")]
use std::sync::Arc;
#[cfg(feature = "desktop")]
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

#[cfg(feature = "desktop")]

struct App {
    window: Option<Arc<Window>>,
    renderer: Option<GpsRenderer>,
    left_dragging: bool,
    right_dragging: bool,
    last_cursor_pos: Option<(f64, f64)>,
    cursor_pos: Option<(f64, f64)>,
    debug_mode: bool,
    activities: Vec<Vec<GpsVertex>>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(
                        WindowAttributes::default()
                            .with_title("renderer")
                            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720)),
                    )
                    .unwrap(),
            );

            let mut renderer = pollster::block_on(GpsRenderer::new(window.clone())).unwrap();

            let total_vertices: usize = self.activities.iter().map(|a| a.len()).sum();
            log::debug!(
                "Loading {} activities with {} total vertices",
                self.activities.len(),
                total_vertices
            );

            // set camera for 3d perspective view
            // position camera at an angle to see both horizontal and vertical dimensions
            let camera = renderer.camera_mut();
            camera.position = [2.0, -1.5, 2.5]; // angled view from side and above
            camera.target = [0.0, 0.0, 0.0];
            camera.up = [0.0, 0.0, 1.0]; // z-up for geographic data
            camera.zoom = 1.0;
            log::debug!(
                "3D Camera - position: {:?}, target: {:?}, up: {:?}",
                camera.position,
                camera.target,
                camera.up
            );

            renderer.load_activities(self.activities.clone());
            renderer.update_camera();

            self.window = Some(window);
            self.renderer = Some(renderer);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => {
                log::debug!("goodbye");
                event_loop.exit()
            }
            WindowEvent::Resized(physical_size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(wgpu::Extent3d {
                        width: physical_size.width,
                        height: physical_size.height,
                        depth_or_array_layers: 1,
                    });
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.update_camera();
                    match renderer.render(self.debug_mode) {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            if let Some(window) = &self.window {
                                let size = window.inner_size();
                                renderer.resize(wgpu::Extent3d {
                                    width: size.width,
                                    height: size.height,
                                    depth_or_array_layers: 1,
                                });
                            }
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(e) => log::error!("Render error: {:?}", e),
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(renderer) = &mut self.renderer {
                    let zoom_amount = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y * 0.1,
                        MouseScrollDelta::PixelDelta(pos) => (pos.y as f32) * 0.001,
                    };

                    // simple dolly: just move camera closer/farther along view direction
                    renderer.camera_mut().dolly(zoom_amount);

                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => match button {
                MouseButton::Left => match state {
                    ElementState::Pressed => {
                        self.left_dragging = true;
                        self.last_cursor_pos = self.cursor_pos;
                    }
                    ElementState::Released => {
                        self.left_dragging = false;
                        self.last_cursor_pos = None;
                    }
                },
                MouseButton::Right => match state {
                    ElementState::Pressed => {
                        self.right_dragging = true;
                        self.last_cursor_pos = self.cursor_pos;
                    }
                    ElementState::Released => {
                        self.right_dragging = false;
                        self.last_cursor_pos = None;
                    }
                },
                _ => {}
            },
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = Some((position.x, position.y));

                if let Some(last_pos) = self.last_cursor_pos {
                    if let Some(renderer) = &mut self.renderer {
                        let dx = (position.x - last_pos.0) as f32;
                        let dy = (position.y - last_pos.1) as f32;

                        let camera = renderer.camera_mut();

                        if self.left_dragging {
                            let sensitivity = 0.002;
                            camera.orbit(dx * sensitivity, -dy * sensitivity);
                        }

                        if self.right_dragging {
                            let sensitivity = 0.003;
                            camera.pan(dx * sensitivity, -dy * sensitivity);
                        }

                        if self.left_dragging || self.right_dragging {
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                        }
                    }
                }

                if self.left_dragging || self.right_dragging {
                    self.last_cursor_pos = Some((position.x, position.y));
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(code),
                        ..
                    },
                ..
            } => match code {
                KeyCode::Digit0 => {
                    self.debug_mode = !self.debug_mode;
                    log::debug!("Debug mode: {}", if self.debug_mode { "ON" } else { "OFF" });
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
                KeyCode::Digit1 => {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.camera_mut().set_top_down_view(3.0);
                        log::debug!("Camera: Top-down view");
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                }
                KeyCode::Digit2 => {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.camera_mut().set_perspective_view(3.0);
                        log::debug!("Camera: 45° perspective view");
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                }
                KeyCode::KeyW => {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.camera_mut().fly_forward(0.02);
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                }
                KeyCode::KeyS => {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.camera_mut().fly_forward(-0.02);
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                }
                KeyCode::KeyA => {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.camera_mut().fly_right(-0.02);
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                }
                KeyCode::KeyD => {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.camera_mut().fly_right(0.02);
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                }
                KeyCode::ArrowUp => {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.camera_mut().pan(0.0, 0.1);
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                }
                KeyCode::ArrowDown => {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.camera_mut().pan(0.0, -0.1);
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                }
                KeyCode::ArrowLeft => {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.camera_mut().pan(-0.1, 0.0);
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                }
                KeyCode::ArrowRight => {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.camera_mut().pan(0.1, 0.0);
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

#[cfg(feature = "desktop")]
fn main() -> anyhow::Result<()> {
    env_logger::init();

    log::info!("Loading GPS data...");
    // Create a Tokio runtime for loading data
    let rt = tokio::runtime::Runtime::new()?;
    let activities = rt.block_on(load_activities());
    let total_vertices: usize = activities.iter().map(|a| a.len()).sum();
    log::info!(
        "Loaded {} activities with {} total vertices",
        activities.len(),
        total_vertices
    );

    let event_loop = EventLoop::new()?;
    let mut app = App {
        window: None,
        renderer: None,
        left_dragging: false,
        right_dragging: false,
        last_cursor_pos: None,
        cursor_pos: None,
        debug_mode: false,
        activities,
    };

    event_loop.run_app(&mut app)?;

    Ok(())
}

#[cfg(not(feature = "desktop"))]
fn main() {
    eprintln!("This binary requires the 'desktop' feature to be enabled.");
    eprintln!("For WASM builds, use the web interface instead.");
}

#[cfg(feature = "desktop")]
async fn load_activities() -> Vec<Vec<GpsVertex>> {
    // Load data from parquet file
    let parquet_path = "data/gps_data.parquet";

    if !std::path::Path::new(parquet_path).exists() {
        panic!(
            "Parquet file not found at {}. Run 'cargo run -p storage' first to generate it.",
            parquet_path
        );
    }

    log::debug!("Loading parquet from {}", parquet_path);
    let store = storage::GpsStore::with_parquet(parquet_path)
        .await
        .expect("Failed to load parquet data");

    let activity_ids = store
        .get_activity_ids()
        .await
        .expect("Failed to get activity IDs");

    if activity_ids.is_empty() {
        let all_points = store.query_all().await.expect("Failed to query all points");
        panic!(
            "No activities found in data! Total points in store: {}",
            all_points.len()
        );
    }

    log::debug!("Found {} activities", activity_ids.len());

    let activity_colors = [
        [1.0, 0.2, 0.2, 1.0],
        [0.2, 0.2, 1.0, 1.0],
        [0.2, 1.0, 0.2, 1.0],
        [1.0, 1.0, 0.2, 1.0],
        [1.0, 0.2, 1.0, 1.0],
        [0.2, 1.0, 1.0, 1.0],
        [1.0, 0.6, 0.2, 1.0],
        [0.6, 0.2, 1.0, 1.0],
    ];

    let mut all_activities = Vec::new();

    for (idx, activity_id) in activity_ids.iter().enumerate() {
        let gps_points = store
            .query_by_activity(activity_id)
            .await
            .expect("Failed to query activity");

        log::debug!(
            "Activity {} [{}]: {} points",
            idx + 1,
            activity_id,
            gps_points.len()
        );

        let color = activity_colors[idx % activity_colors.len()];

        let vertices: Vec<GpsVertex> = gps_points
            .iter()
            .map(|point| {
                GpsVertex::from_lat_lon_alt(point.latitude, point.longitude, point.altitude, color)
            })
            .collect();

        all_activities.push(vertices);
    }

    let mut all_vertices_flat = all_activities.iter().flatten().copied().collect::<Vec<_>>();
    log::debug!("Normalizing {} total vertices", all_vertices_flat.len());
    GpsVertex::normalize_batch(&mut all_vertices_flat);

    let mut result = Vec::new();
    let mut offset = 0;
    for activity in &all_activities {
        let count = activity.len();
        result.push(all_vertices_flat[offset..offset + count].to_vec());
        offset += count;
    }

    log::debug!("Created {} separate activities for rendering", result.len());

    result
}
