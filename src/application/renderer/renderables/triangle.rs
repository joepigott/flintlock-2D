use super::vertices::*;

use nalgebra_glm::TMat4;

/// Triangle with texture data.
pub struct Triangle {
    vertices: [Vertex2D; 3],
    matrix: TMat4<f32>,
}

impl super::Renderable for Triangle {
    fn vertices(&self) -> Vec<super::vertices::Vertex2D> {
        self.vertices.into()
    }

    fn matrix(&self) -> TMat4<f32> {
        self.matrix
    }
}

/// Triangle with color data. No texture data.
pub struct ColorTriangle {
    vertices: [ColorVertex2D; 3],
    matrix: TMat4<f32>,
}

impl super::ColorRenderable for ColorTriangle {
    fn vertices(&self) -> Vec<super::vertices::ColorVertex2D> {
        self.vertices.into()
    }

    fn matrix(&self) -> TMat4<f32> {
        self.matrix
    }
}
