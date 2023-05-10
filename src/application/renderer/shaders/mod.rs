use std::sync::Arc;

use vulkano::pipeline::graphics::GraphicsPipeline;

pub struct PipelineInfo {
    pub vert_path: String,
    pub frag_path: String,
    pub pipeline: Arc<GraphicsPipeline>
}

pub mod deferred_vert {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/application/renderer/shaders/shaders/deferred.vert",
    }
}

pub mod deferred_frag {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/application/renderer/shaders/shaders/deferred.frag"
    }
}

pub mod ambient_vert {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/application/renderer/shaders/shaders/ambient.vert"
    }
}

pub mod ambient_frag {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/application/renderer/shaders/shaders/ambient.frag"
    }
}

pub mod point_vert {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/application/renderer/shaders/shaders/point.vert"
    }
}

pub mod point_frag {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/application/renderer/shaders/shaders/point.frag"
    }
}

pub mod directional_vert {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/application/renderer/shaders/shaders/directional.vert"
    }
}

pub mod directional_frag {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/application/renderer/shaders/shaders/directional.frag"
    }
}
