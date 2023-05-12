use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct Vertex2D {
    position: [f32; 3],
    uv: [f32; 2],
}
vulkano::impl_vertex!(Vertex2D, position, uv);

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct BasicVertex2D {
    position: [f32; 2],
}
vulkano::impl_vertex!(BasicVertex2D, position);

impl BasicVertex2D {
    pub fn screen_vertices() -> [BasicVertex2D; 6] {
        [
            BasicVertex2D {
                position: [-1.0, -1.0],
            },
            BasicVertex2D {
                position: [-1.0, 1.0],
            },
            BasicVertex2D {
                position: [1.0, 1.0],
            },
            BasicVertex2D {
                position: [-1.0, -1.0],
            },
            BasicVertex2D {
                position: [1.0, 1.0],
            },
            BasicVertex2D {
                position: [1.0, -1.0],
            },
        ]
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct ColorVertex2D {
    position: [f32; 3],
    color: [f32; 3],
}
vulkano::impl_vertex!(ColorVertex2D, position, color);
