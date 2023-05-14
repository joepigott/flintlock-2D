use super::vertices::*;

use nalgebra_glm::{TMat4, Vec3, translate, rotate};

/// Triangle with texture data.
pub struct Triangle {
    pub vertices: [Vertex2D; 3],
    pub translation: TMat4<f32>,
    pub rotation: TMat4<f32>,
}

impl super::Renderable for Triangle {
    fn vertices(&self) -> Vec<super::vertices::Vertex2D> {
        self.vertices.into()
    }

    fn matrix(&self) -> TMat4<f32> {
        self.translation * self.rotation
    }

    fn translate(&mut self, translation: Vec3) {
        self.translation = translate(&self.translation, &translation);
    }

    fn rotate(&mut self, radians: f32, axis: Vec3) {
        self.rotation = rotate(&self.rotation, radians, &axis);
    }
}

/// Triangle with color data. No texture data.
pub struct ColorTriangle {
    pub vertices: [ColorVertex2D; 3],
    pub translation: TMat4<f32>,
    pub rotation: TMat4<f32>,
}

impl super::ColorRenderable for ColorTriangle {
    fn vertices(&self) -> Vec<super::vertices::ColorVertex2D> {
        self.vertices.into()
    }

    fn matrix(&self) -> TMat4<f32> {
        self.translation * self.rotation
    }

    fn translate(&mut self, translation: Vec3) {
        self.translation = translate(&self.translation, &translation);
    }

    fn rotate(&mut self, radians: f32, axis: Vec3) {
        self.rotation = rotate(&self.rotation, radians, &axis);
    }
}
