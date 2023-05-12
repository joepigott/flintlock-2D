pub mod lights;
pub mod quad;
pub mod triangle;
pub mod vertices;

use nalgebra_glm::TMat4;

pub trait Renderable {
    fn vertices(&self) -> Vec<vertices::Vertex2D>;
    fn matrix(&self) -> TMat4<f32>;
}

pub trait ColorRenderable {
    fn vertices(&self) -> Vec<vertices::ColorVertex2D>;
    fn matrix(&self) -> TMat4<f32>;
}
