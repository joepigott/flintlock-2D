use super::vertices::*;

use nalgebra_glm::TMat4;

/// Quad with texture data.
pub struct Quad {
    pub vertices: [Vertex2D; 6],
    pub matrix: TMat4<f32>,
}

impl super::Renderable for Quad {
    fn vertices(&self) -> Vec<Vertex2D> {
        self.vertices.into()
    }

    fn matrix(&self) -> TMat4<f32> {
        self.matrix
    }
}

/// Quad with color data. No texture data.
pub struct ColorQuad {
    pub vertices: [ColorVertex2D; 6],
    pub matrix: TMat4<f32>,
}

impl super::ColorRenderable for ColorQuad {
    fn vertices(&self) -> Vec<super::vertices::ColorVertex2D> {
        self.vertices.into()
    }

    fn matrix(&self) -> TMat4<f32> {
        self.matrix
    }
}
