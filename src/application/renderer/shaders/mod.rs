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

pub mod point_light_vert {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/application/renderer/shaders/shaders/point_light.vert"
    }
}

pub mod point_light_frag {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/application/renderer/shaders/shaders/point_light.frag"
    }
}

pub mod directional_light_vert {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/application/renderer/shaders/shaders/directional_light.vert"
    }
}

pub mod directional_light_frag {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/application/renderer/shaders/shaders/directional_light.frag"
    }
}
