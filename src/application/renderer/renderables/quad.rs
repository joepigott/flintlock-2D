use super::vertices::*;

use nalgebra_glm::{TMat4, Vec3, translate, rotate};

/// Quad with texture data.
pub struct Quad {
    pub vertices: [Vertex2D; 6],
    pub translation: TMat4<f32>,
    pub rotation: TMat4<f32>
}

impl super::Renderable for Quad {
    fn vertices(&self) -> Vec<Vertex2D> {
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

    fn reset_rotation(&mut self) {
        self.rotation = nalgebra_glm::identity();
    }
}

/// Quad with color data. No texture data.
pub struct ColorQuad {
    pub vertices: [ColorVertex2D; 6],
    pub translation: TMat4<f32>,
    pub rotation: TMat4<f32>
}

impl super::ColorRenderable for ColorQuad {
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

    fn reset_rotation(&mut self) {
        self.rotation = nalgebra_glm::identity();
    }
}
