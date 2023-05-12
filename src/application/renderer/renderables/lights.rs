pub struct DirectionalLight {
    pub direction: [f32; 2],
    pub color: [f32; 3],
    pub intensity: f32,
}

pub struct PointLight {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
}
