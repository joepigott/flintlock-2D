pub mod lights;
pub mod quad;
pub mod triangle;
pub mod vertices;

use nalgebra_glm::{TMat4, Vec3};

pub trait Renderable {
    fn vertices(&self) -> Vec<vertices::Vertex2D>;
    fn matrix(&self) -> TMat4<f32>;
    fn translate(&mut self, translation: Vec3);
    fn rotate(&mut self, radians: f32, axis: Vec3);
    fn reset_rotation(&mut self);
}

pub trait ColorRenderable {
    fn vertices(&self) -> Vec<vertices::ColorVertex2D>;
    fn matrix(&self) -> TMat4<f32>;
    fn translate(&mut self, translation: Vec3);
    fn rotate(&mut self, radians: f32, axis: Vec3);
    fn reset_rotation(&mut self);
}
