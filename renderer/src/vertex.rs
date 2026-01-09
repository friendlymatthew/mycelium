use bytemuck::{Pod, Zeroable};

pub type RGBA = [f32; 4];

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GpsVertex {
    /// position in 3d space (lon, lat, altitude)
    pub position: [f32; 3],
    pub color: RGBA,
}

impl GpsVertex {
    pub fn from_lat_lon_alt(lat: f64, lon: f64, altitude: Option<f32>, color: [f32; 4]) -> Self {
        Self {
            position: [lon as f32, lat as f32, altitude.unwrap_or(0.0)],
            color,
        }
    }

    pub fn normalize_batch(vertices: &mut [GpsVertex]) {
        if vertices.is_empty() {
            return;
        }

        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        let mut min_z = f32::MAX;
        let mut max_z = f32::MIN;

        for v in vertices.iter() {
            min_x = min_x.min(v.position[0]);
            max_x = max_x.max(v.position[0]);
            min_y = min_y.min(v.position[1]);
            max_y = max_y.max(v.position[1]);
            min_z = min_z.min(v.position[2]);
            max_z = max_z.max(v.position[2]);
        }

        log::debug!(
            "Bounds before normalization: x [{}, {}], y [{}, {}], z [{}, {}]",
            min_x,
            max_x,
            min_y,
            max_y,
            min_z,
            max_z
        );

        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;
        let center_z = (min_z + max_z) / 2.0;
        let span_x = max_x - min_x;
        let span_y = max_y - min_y;

        let max_span = span_x.max(span_y);
        let scale = 1.8 / max_span;

        // apply some vertical exageration to make the altitude visible
        // lat/lon degrees are ~111km per deg
        let altitude_scale = scale / 111_000.0;

        // todo: make this into a constant
        let vertical_exaggeration = 10.0;

        log::debug!(
            "Normalizing: center ({}, {}, {}), horizontal span {}, scale {}, altitude scale {} (with {}x exaggeration)",
            center_x,
            center_y,
            center_z,
            max_span,
            scale,
            altitude_scale * 10.0,
            vertical_exaggeration
        );

        for v in vertices.iter_mut() {
            v.position[0] = (v.position[0] - center_x) * scale;
            v.position[1] = (v.position[1] - center_y) * scale;
            v.position[2] = (v.position[2] - center_z) * altitude_scale * vertical_exaggeration;
        }
    }

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GpsVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}
