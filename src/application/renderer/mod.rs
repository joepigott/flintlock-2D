// This file contains code copied and/or adapted from code provided by the
// Vulkano project and the Vulkano tutorial by GitHub user taidaesal, both 
// under the MIT license.
//
// Vulkano => https://vulkano.rs
// Vulkano Tutorial => https://github.com/taidaesal/vulkano_tutorial

use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, CpuBufferPool, TypedBufferAccess};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, SubpassContents,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::device::{Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{AttachmentImage, ImageAccess, SwapchainImage};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::pipeline::graphics::color_blend::{
    AttachmentBlend, BlendFactor, BlendOp, ColorBlendState,
};
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::rasterization::{CullMode, RasterizationState};
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{GraphicsPipeline, Pipeline};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::swapchain::{
    self, AcquireError, Surface, Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo,
    SwapchainCreationError, SwapchainPresentInfo,
};
use vulkano::sync::GpuFuture;
use vulkano::VulkanLibrary;

use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

// VkSurfaceBuild allows winit to build a vulkan surface directly
use vulkano_win::{required_extensions, VkSurfaceBuild};

use nalgebra_glm::{identity, TMat4, perspective, half_pi};

use std::sync::Arc;

mod shaders;
use shaders::*;

pub mod renderables;
use renderables::lights::*;
use renderables::vertices::*;

// render stage allows renderer to function as state machine
enum RenderStage {
    Stopped,
    Vertex,
    Ambient,
    Point,
    Directional,
    RedrawNeeded,
}

struct VP {
    view: TMat4<f32>,
    projection: TMat4<f32>
}

impl VP {
    fn new() -> VP {
        VP {
            view: identity(),
            projection: identity()
        }
    }
}

pub struct Renderer {
    surface: Arc<Surface>,
    pub device: Arc<Device>,
    queue: Arc<Queue>,
    vp: VP,
    swapchain: Arc<Swapchain>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    descriptor_set_allocator: StandardDescriptorSetAllocator,
    command_buffer_allocator: StandardCommandBufferAllocator,
    render_pass: Arc<RenderPass>,
    deferred_pipeline: PipelineInfo,
    ambient_pipeline: PipelineInfo,
    point_pipeline: PipelineInfo,
    directional_pipeline: PipelineInfo,
    vp_buffer: Arc<CpuAccessibleBuffer<deferred_vert::ty::VPData>>,
    model_uniform_buffer: CpuBufferPool<deferred_vert::ty::ModelData>,
    ambient_buffer: Arc<CpuAccessibleBuffer<ambient_frag::ty::AmbientData>>,
    point_buffer: CpuBufferPool<point_frag::ty::PointData>,
    directional_buffer: CpuBufferPool<directional_frag::ty::DirectionalData>,
    screen_vertices: Arc<CpuAccessibleBuffer<[BasicVertex2D]>>,
    vp_set: Arc<PersistentDescriptorSet>,
    viewport: Viewport,
    framebuffers: Vec<Arc<Framebuffer>>,
    color_buffer: Arc<ImageView<AttachmentImage>>,
    commands: Option<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>>,
    image_index: u32,
    acquire_future: Option<SwapchainAcquireFuture>,
    render_stage: RenderStage,
}

impl Renderer {
    /// Initializes a new Vulkan program and returns a Renderer instance.
    pub fn new(event_loop: &EventLoop<()>) -> Renderer {
        // vulkan instance. vulkano takes care of most of the configuration
        let instance = {
            let library = VulkanLibrary::new().unwrap();
            let extensions = required_extensions(&library);

            Instance::new(
                library,
                InstanceCreateInfo {
                    enabled_extensions: extensions,
                    enumerate_portability: true, // allows porting to macOS
                    max_api_version: Some(vulkano::Version::V1_1),
                    ..Default::default()
                },
            )
            .unwrap()
        };

        // surface to render to. provided by winit, helper function by vulkano
        let surface = WindowBuilder::new()
            .build_vk_surface(event_loop, instance.clone())
            .unwrap();

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        println!("Available devices:");
        for device in instance.enumerate_physical_devices().unwrap() {
            println!("\t{}", device.properties().device_name);
        }

        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()
            .unwrap()
            .filter(|device| device.supported_extensions().contains(&device_extensions))
            .filter_map(|device| {
                device
                    .queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        // pick the first queue index that can handle graphics
                        q.queue_flags.graphics
                            && device.surface_support(i as u32, &surface).unwrap_or(false)
                    })
                    .map(|i| (device, i as u32))
            })
            .min_by_key(|(device, _)| {
                // pick the best available graphics unit
                match device.properties().device_type {
                    PhysicalDeviceType::DiscreteGpu => 0,
                    PhysicalDeviceType::IntegratedGpu => 1,
                    PhysicalDeviceType::VirtualGpu => 2,
                    PhysicalDeviceType::Cpu => 3,
                    PhysicalDeviceType::Other => 4,
                    _ => 5,
                }
            })
            .expect("No suitable GPU found.");

        println!("Using device {}", physical_device.properties().device_name);

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: device_extensions,
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
        .unwrap();

        let queue = queues.next().unwrap();

        let mut vp = VP::new();

        let (swapchain, images) = {
            let caps = device
                .physical_device()
                .surface_capabilities(&surface, Default::default())
                .unwrap();

            let image_usage = caps.supported_usage_flags;
            let composite_alpha = caps.supported_composite_alpha.iter().next().unwrap();

            let image_format = Some(
                device
                    .physical_device()
                    .surface_formats(&surface, Default::default())
                    .unwrap()[0]
                    .0,
            );

            let window = surface.object().unwrap().downcast_ref::<Window>().unwrap();
            let image_extent: [u32; 2] = window.inner_size().into();

            // let aspect_ratio = image_extent[0] as f32 / image_extent[1] as f32;
            // vp.projection = perspective(aspect_ratio, half_pi(), 0.01, 100.0);
            vp.projection = nalgebra_glm::ortho(
                -0.0025 * image_extent[0] as f32,
                0.0025  * image_extent[0] as f32,
                -0.0025 * image_extent[1] as f32,
                0.0025  * image_extent[1] as f32,
                -100.0,
                100.0
            );

            Swapchain::new(
                device.clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: caps.min_image_count,
                    image_format,
                    image_extent,
                    image_usage,
                    composite_alpha,
                    ..Default::default()
                },
            )
            .unwrap()
        };

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let descriptor_set_allocator = StandardDescriptorSetAllocator::new(device.clone());
        let command_buffer_allocator =
            StandardCommandBufferAllocator::new(device.clone(), Default::default());

        let deferred_vert = deferred_vert::load(device.clone()).unwrap();
        let deferred_frag = deferred_frag::load(device.clone()).unwrap();
        let ambient_vert = ambient_vert::load(device.clone()).unwrap();
        let ambient_frag = ambient_frag::load(device.clone()).unwrap();
        let point_vert = point_vert::load(device.clone()).unwrap();
        let point_frag = point_frag::load(device.clone()).unwrap();
        let directional_vert = directional_vert::load(device.clone()).unwrap();
        let directional_frag = directional_frag::load(device.clone()).unwrap();

        let render_pass = vulkano::ordered_passes_renderpass!(device.clone(),
            attachments: {
                final_color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.image_format(),
                    samples: 1,
                },
                color: {
                    load: Clear,
                    store: DontCare,
                    format: Format::A2B10G10R10_UNORM_PACK32,
                    samples: 1,
                },
                depth: {
                    load: Clear,
                    store: DontCare,
                    format: Format::D16_UNORM,
                    samples: 1,
                }
            },
            passes: [
                {
                    color: [color],
                    depth_stencil: {depth},
                    input: []
                },
                {
                    color: [final_color],
                    depth_stencil: {},
                    input: [color]
                }
            ]
        )
        .unwrap();

        let deferred_pass = Subpass::from(render_pass.clone(), 0).unwrap();
        let lighting_pass = Subpass::from(render_pass.clone(), 1).unwrap();

        let deferred_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<ColorVertex2D>())
            .vertex_shader(deferred_vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(deferred_frag.entry_point("main").unwrap(), ())
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
            .render_pass(deferred_pass.clone())
            .build(device.clone())
            .unwrap();

        let ambient_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<BasicVertex2D>())
            .vertex_shader(ambient_vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(ambient_frag.entry_point("main").unwrap(), ())
            .color_blend_state(
                ColorBlendState::new(lighting_pass.num_color_attachments()).blend(
                    AttachmentBlend {
                        color_op: BlendOp::Add,
                        color_source: BlendFactor::One,
                        color_destination: BlendFactor::One,
                        alpha_op: BlendOp::Add,
                        alpha_source: BlendFactor::One,
                        alpha_destination: BlendFactor::One,
                    },
                ),
            )
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
            .render_pass(lighting_pass.clone())
            .build(device.clone())
            .unwrap();

        let point_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<BasicVertex2D>())
            .vertex_shader(point_vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(point_frag.entry_point("main").unwrap(), ())
            .color_blend_state(
                ColorBlendState::new(lighting_pass.num_color_attachments()).blend(
                    AttachmentBlend {
                        color_op: BlendOp::Add,
                        color_source: BlendFactor::One,
                        color_destination: BlendFactor::One,
                        alpha_op: BlendOp::Add,
                        alpha_source: BlendFactor::One,
                        alpha_destination: BlendFactor::One,
                    },
                ),
            )
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
            .render_pass(lighting_pass.clone())
            .build(device.clone())
            .unwrap();

        let directional_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<BasicVertex2D>())
            .vertex_shader(directional_vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(directional_frag.entry_point("main").unwrap(), ())
            .color_blend_state(
                ColorBlendState::new(lighting_pass.num_color_attachments()).blend(
                    AttachmentBlend {
                        color_op: BlendOp::Add,
                        color_source: BlendFactor::One,
                        color_destination: BlendFactor::One,
                        alpha_op: BlendOp::Add,
                        alpha_source: BlendFactor::One,
                        alpha_destination: BlendFactor::One,
                    },
                ),
            )
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
            .render_pass(lighting_pass.clone())
            .build(device.clone())
            .unwrap();

        let deferred_pipeline = PipelineInfo {
            vert_path: "src/application/renderer/shaders/shaders/deferred.vert".to_string(),
            frag_path: "src/application/renderer/shaders/shaders/deferred.frag".to_string(),
            pipeline: deferred_pipeline,
        };

        let ambient_pipeline = PipelineInfo {
            vert_path: "src/application/renderer/shaders/shaders/ambient.vert".to_string(),
            frag_path: "src/application/renderer/shaders/shaders/ambient.frag".to_string(),
            pipeline: ambient_pipeline,
        };

        let point_pipeline = PipelineInfo {
            vert_path: "src/application/renderer/shaders/shaders/point.vert".to_string(),
            frag_path: "src/application/renderer/shaders/shaders/point.frag".to_string(),
            pipeline: point_pipeline,
        };

        let directional_pipeline = PipelineInfo {
            vert_path: "src/application/renderer/shaders/shaders/directional.vert".to_string(),
            frag_path: "src/application/renderer/shaders/shaders/directional.frag".to_string(),
            pipeline: directional_pipeline,
        };

        // buffers
        
        let vp_buffer = CpuAccessibleBuffer::from_data(
            &memory_allocator,
            BufferUsage {
                uniform_buffer: true,
                ..BufferUsage::empty()
            },
            false,
            deferred_vert::ty::VPData {
                view: vp.view.into(),
                projection: vp.projection.into()
            }
        )
        .unwrap();

        let model_uniform_buffer: CpuBufferPool<deferred_vert::ty::ModelData> =
            CpuBufferPool::uniform_buffer(memory_allocator.clone());

        let ambient_buffer = CpuAccessibleBuffer::from_data(
            &memory_allocator,
            BufferUsage {
                uniform_buffer: true,
                ..BufferUsage::empty()
            },
            false,
            // default ambient light values
            ambient_frag::ty::AmbientData {
                color: [1.0, 1.0, 1.0],
                intensity: 0.1,
            },
        )
        .unwrap();

        let point_buffer: CpuBufferPool<point_frag::ty::PointData> =
            CpuBufferPool::uniform_buffer(memory_allocator.clone());
        let directional_buffer: CpuBufferPool<directional_frag::ty::DirectionalData> =
            CpuBufferPool::uniform_buffer(memory_allocator.clone());

        // screen vertices allow fragment shaders to execute without vertex data

        let screen_vertices = CpuAccessibleBuffer::from_iter(
            &memory_allocator,
            BufferUsage {
                vertex_buffer: true,
                ..BufferUsage::empty()
            },
            false,
            BasicVertex2D::screen_vertices().iter().cloned(),
        )
        .unwrap();

        let vp_layout = deferred_pipeline.pipeline
            .layout()
            .set_layouts()
            .get(0)
            .unwrap();
        let vp_set = PersistentDescriptorSet::new(
            &descriptor_set_allocator,
            vp_layout.clone(),
            [WriteDescriptorSet::buffer(0, vp_buffer.clone())]
        )
        .unwrap();

        let mut viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [0.0, 0.0],
            depth_range: 0.0..1.0,
        };

        let (framebuffers, color_buffer) = Renderer::window_size_dependent_setup(
            &memory_allocator,
            &images,
            render_pass.clone(),
            &mut viewport,
        );

        let commands: Option<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>> = None;
        let image_index = 0;
        let acquire_future: Option<SwapchainAcquireFuture> = None;

        let render_stage = RenderStage::Stopped;

        Renderer {
            surface,
            device,
            queue,
            vp,
            swapchain,
            memory_allocator,
            descriptor_set_allocator,
            command_buffer_allocator,
            render_pass,
            deferred_pipeline,
            ambient_pipeline,
            point_pipeline,
            directional_pipeline,
            vp_buffer,
            model_uniform_buffer,
            ambient_buffer,
            point_buffer,
            directional_buffer,
            screen_vertices,
            vp_set,
            viewport,
            framebuffers,
            color_buffer,
            commands,
            image_index,
            acquire_future,
            render_stage,
        }
    }

    /// Creates a command buffer and prepares the system for rendering.
    pub fn start(&mut self) {
        match self.render_stage {
            RenderStage::Stopped => {
                self.render_stage = RenderStage::Vertex;
            }
            RenderStage::RedrawNeeded => {
                self.recreate_swapchain();
                self.render_stage = RenderStage::Stopped;
                self.commands = None;
                return;
            }
            _ => {
                self.render_stage = RenderStage::Stopped;
                self.commands = None;
                return;
            }
        }

        let (image_index, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.recreate_swapchain();
                    return;
                }
                Err(err) => {
                    panic!("{}", err);
                }
            };

        if suboptimal {
            self.recreate_swapchain();
            return;
        }

        let clear_values = vec![
            Some([0.01098, 0.01059, 0.00902, 1.0].into()),
            Some([0.01098, 0.01059, 0.00902, 1.0].into()),
            Some(1.0.into()),
        ];

        let mut commands = AutoCommandBufferBuilder::primary(
            &self.command_buffer_allocator,
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        commands
            .begin_render_pass(
                vulkano::command_buffer::RenderPassBeginInfo {
                    clear_values,
                    ..vulkano::command_buffer::RenderPassBeginInfo::framebuffer(
                        self.framebuffers[image_index as usize].clone(),
                    )
                },
                SubpassContents::Inline,
            )
            .unwrap();

        self.commands = Some(commands);
        self.image_index = image_index;
        self.acquire_future = Some(acquire_future);
    }

    pub fn draw(&mut self, model: &dyn renderables::Renderable) {
        match self.render_stage {
            RenderStage::Vertex => {}
            RenderStage::RedrawNeeded => {
                self.recreate_swapchain();
                self.commands = None;
                self.render_stage = RenderStage::Stopped;
                return;
            }
            _ => {
                self.commands = None;
                self.render_stage = RenderStage::Stopped;
                return;
            }
        }

        let model_subbuffer = {
            let model_mat = model.matrix();

            let uniform_data = deferred_vert::ty::ModelData {
                mat: model_mat.into(),
            };

            self.model_uniform_buffer.from_data(uniform_data).unwrap()
        };

        let model_layout = self
            .deferred_pipeline
            .pipeline
            .layout()
            .set_layouts()
            .get(1)
            .unwrap();
        let model_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            model_layout.clone(),
            [WriteDescriptorSet::buffer(0, model_subbuffer.clone())],
        )
        .unwrap();

        let vertex_buffer = CpuAccessibleBuffer::from_iter(
            &self.memory_allocator,
            BufferUsage {
                vertex_buffer: true,
                ..BufferUsage::empty()
            },
            false,
            model.vertices().iter().cloned(),
        )
        .unwrap();

        self.commands
            .as_mut()
            .unwrap()
            .set_viewport(0, [self.viewport.clone()])
            .bind_pipeline_graphics(self.deferred_pipeline.pipeline.clone())
            .bind_descriptor_sets(
                vulkano::pipeline::PipelineBindPoint::Graphics,
                self.deferred_pipeline.pipeline.layout().clone(),
                0,
                (self.vp_set.clone(), model_set.clone())
            )
            .bind_vertex_buffers(0, vertex_buffer.clone())
            .draw(vertex_buffer.len() as u32, 1, 0, 0)
            .unwrap();
    }

    pub fn color_draw(&mut self, model: &dyn renderables::ColorRenderable) {
        match self.render_stage {
            RenderStage::Vertex => {}
            RenderStage::RedrawNeeded => {
                self.recreate_swapchain();
                self.commands = None;
                self.render_stage = RenderStage::Stopped;
                return;
            }
            _ => {
                self.commands = None;
                self.render_stage = RenderStage::Stopped;
                return;
            }
        }

        let model_subbuffer = {
            let uniform_data = deferred_vert::ty::ModelData {
                mat: model.matrix().into(),
            };

            self.model_uniform_buffer.from_data(uniform_data).unwrap()
        };

        let model_layout = self
            .deferred_pipeline
            .pipeline
            .layout()
            .set_layouts()
            .get(0)
            .unwrap();
        let model_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            model_layout.clone(),
            [WriteDescriptorSet::buffer(0, model_subbuffer.clone())],
        )
        .unwrap();

        let vertex_buffer = CpuAccessibleBuffer::from_iter(
            &self.memory_allocator,
            BufferUsage {
                vertex_buffer: true,
                ..BufferUsage::empty()
            },
            false,
            model.vertices().iter().cloned(),
        )
        .unwrap();

        self.commands
            .as_mut()
            .unwrap()
            .set_viewport(0, [self.viewport.clone()])
            .bind_pipeline_graphics(self.deferred_pipeline.pipeline.clone())
            .bind_descriptor_sets(
                vulkano::pipeline::PipelineBindPoint::Graphics,
                self.deferred_pipeline.pipeline.layout().clone(),
                0,
                (self.vp_set.clone(), model_set.clone())
            )
            .bind_vertex_buffers(0, vertex_buffer.clone())
            .draw(vertex_buffer.len() as u32, 1, 0, 0)
            .unwrap();
    }

    /// Executes the ambient render stage.
    /// * This function provides the only path out of the vertex render stage,
    /// and must be executed before any other lighting stages.
    pub fn ambient(&mut self) {
        match self.render_stage {
            RenderStage::Vertex => {
                self.render_stage = RenderStage::Ambient;
            }
            RenderStage::Ambient => {
                return;
            }
            RenderStage::RedrawNeeded => {
                self.recreate_swapchain();
                self.commands = None;
                self.render_stage = RenderStage::Stopped;
                return;
            }
            _ => {
                self.commands = None;
                self.render_stage = RenderStage::Stopped;
                return;
            }
        }

        let ambient_layout = self
            .ambient_pipeline
            .pipeline
            .layout()
            .set_layouts()
            .get(0)
            .unwrap();
        let ambient_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            ambient_layout.clone(),
            [
                WriteDescriptorSet::image_view(0, self.color_buffer.clone()),
                WriteDescriptorSet::buffer(1, self.ambient_buffer.clone()),
            ],
        )
        .unwrap();

        self.commands
            .as_mut()
            .unwrap()
            .next_subpass(SubpassContents::Inline)
            .unwrap()
            .bind_pipeline_graphics(self.ambient_pipeline.pipeline.clone())
            .bind_descriptor_sets(
                vulkano::pipeline::PipelineBindPoint::Graphics,
                self.ambient_pipeline.pipeline.layout().clone(),
                0,
                ambient_set.clone(),
            )
            .set_viewport(0, [self.viewport.clone()])
            .bind_vertex_buffers(0, self.screen_vertices.clone())
            .draw(self.screen_vertices.len() as u32, 1, 0, 0)
            .unwrap();
    }

    pub fn set_ambient(&mut self, color: [f32; 3], intensity: f32) {
        self.ambient_buffer = CpuAccessibleBuffer::from_data(
            &self.memory_allocator,
            BufferUsage {
                uniform_buffer: true,
                ..BufferUsage::empty()
            },
            false,
            ambient_frag::ty::AmbientData { color, intensity },
        )
        .unwrap();
    }

    /// Draws a given DirectionalLight.
    pub fn directional(&mut self, light: &DirectionalLight) {
        match self.render_stage {
            RenderStage::Ambient => {
                self.render_stage = RenderStage::Directional;
            }
            RenderStage::Directional => {}
            RenderStage::RedrawNeeded => {
                self.recreate_swapchain();
                self.commands = None;
                self.render_stage = RenderStage::Stopped;
                return;
            }
            _ => {
                self.commands = None;
                self.render_stage = RenderStage::Stopped;
                return;
            }
        }

        let directional_subbuffer = {
            let uniform_data = directional_frag::ty::DirectionalData {
                direction: light.direction.into(),
                color: light.color.into(),
                intensity: light.intensity.into(),
                _dummy0: [0; 8],
            };

            self.directional_buffer.from_data(uniform_data).unwrap()
        };

        let directional_layout = self
            .directional_pipeline
            .pipeline
            .layout()
            .set_layouts()
            .get(0)
            .unwrap();
        let directional_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            directional_layout.clone(),
            [
                WriteDescriptorSet::image_view(0, self.color_buffer.clone()),
                WriteDescriptorSet::buffer(1, directional_subbuffer.clone()),
            ],
        )
        .unwrap();

        self.commands
            .as_mut()
            .unwrap()
            .set_viewport(0, [self.viewport.clone()])
            .bind_pipeline_graphics(self.directional_pipeline.pipeline.clone())
            .bind_vertex_buffers(0, self.screen_vertices.clone())
            .bind_descriptor_sets(
                vulkano::pipeline::PipelineBindPoint::Graphics,
                self.directional_pipeline.pipeline.layout().clone(),
                0,
                directional_set.clone(),
            )
            .draw(self.screen_vertices.len() as u32, 1, 0, 0)
            .unwrap();
    }

    pub fn point(&mut self, light: &PointLight) {
        match self.render_stage {
            RenderStage::Ambient => {
                self.render_stage = RenderStage::Point;
            }
            RenderStage::Directional => {
                self.render_stage = RenderStage::Point;
            }
            RenderStage::Point => {}
            RenderStage::RedrawNeeded => {
                self.recreate_swapchain();
                self.commands = None;
                self.render_stage = RenderStage::Stopped;
                return;
            }
            _ => {
                self.commands = None;
                self.render_stage = RenderStage::Stopped;
                return;
            }
        }

        let point_subbuffer = {
            let uniform_data = point_frag::ty::PointData {
                position: light.position.into(),
                color: light.color.into(),
                intensity: light.intensity.into(),
                _dummy0: [0; 4],
            };

            self.point_buffer.from_data(uniform_data).unwrap()
        };

        let point_layout = self
            .point_pipeline
            .pipeline
            .layout()
            .set_layouts()
            .get(0)
            .unwrap();
        let point_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            point_layout.clone(),
            [
                WriteDescriptorSet::image_view(0, self.color_buffer.clone()),
                WriteDescriptorSet::buffer(1, point_subbuffer.clone()),
            ],
        )
        .unwrap();

        self.commands
            .as_mut()
            .unwrap()
            .set_viewport(0, [self.viewport.clone()])
            .bind_pipeline_graphics(self.point_pipeline.pipeline.clone())
            .bind_vertex_buffers(0, self.screen_vertices.clone())
            .bind_descriptor_sets(
                vulkano::pipeline::PipelineBindPoint::Graphics,
                self.point_pipeline.pipeline.layout().clone(),
                0,
                point_set.clone(),
            )
            .draw(self.screen_vertices.len() as u32, 1, 0, 0)
            .unwrap();
    }

    pub fn set_view(&mut self, view: &TMat4<f32>) {
        self.vp.view = view.clone();
        self.vp_buffer = CpuAccessibleBuffer::from_data(
            &self.memory_allocator,
            BufferUsage {
                uniform_buffer: true,
                ..BufferUsage::empty()
            },
            false,
            deferred_vert::ty::VPData {
                view: self.vp.view.clone().into(),
                projection: self.vp.projection.clone().into()
            }
        )
        .unwrap();

        let vp_layout = self.deferred_pipeline.pipeline
            .layout()
            .set_layouts()
            .get(0)
            .unwrap();
        self.vp_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            vp_layout.clone(),
            [WriteDescriptorSet::buffer(0, self.vp_buffer.clone())]
        )
        .unwrap();

        self.render_stage = RenderStage::Stopped;
    }

    pub fn recreate_swapchain(&mut self) {
        self.render_stage = RenderStage::RedrawNeeded;
        self.commands = None;

        let window = self
            .surface
            .object()
            .unwrap()
            .downcast_ref::<Window>()
            .unwrap();
        let image_extent: [u32; 2] = window.inner_size().into();

        // let aspect_ratio = image_extent[0] as f32 / image_extent[1] as f32;
        // self.vp.projection = perspective(aspect_ratio, half_pi(), 0.01, 100.0);
        self.vp.projection = nalgebra_glm::ortho(
            -0.0025 * image_extent[0] as f32,
            0.0025  * image_extent[0] as f32,
            -0.0025 * image_extent[1] as f32,
            0.0025  * image_extent[1] as f32,
            -100.0,
            100.0
        );

        let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
            image_extent,
            ..self.swapchain.create_info()
        }) {
            Ok(r) => r,
            Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
            Err(err) => panic!("Failed to recreate swapchain: {:?}", err),
        };

        let (new_framebuffers, new_color_buffer) = Renderer::window_size_dependent_setup(
            &self.memory_allocator,
            &new_images,
            self.render_pass.clone(),
            &mut self.viewport,
        );

        self.swapchain = new_swapchain;
        self.framebuffers = new_framebuffers;
        self.color_buffer = new_color_buffer;

        self.vp_buffer = CpuAccessibleBuffer::from_data(
            &self.memory_allocator,
            BufferUsage {
                uniform_buffer: true,
                ..BufferUsage::empty()
            },
            false,
            deferred_vert::ty::VPData {
                view: self.vp.view.into(),
                projection: self.vp.projection.into()
            }
        )
        .unwrap();

        let vp_layout = self.deferred_pipeline.pipeline
            .layout()
            .set_layouts()
            .get(0)
            .unwrap();
        self.vp_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            vp_layout.clone(),
            [WriteDescriptorSet::buffer(0, self.vp_buffer.clone())]
        )
        .unwrap();

        self.render_stage = RenderStage::Stopped;
    }

    pub fn render(&mut self, previous_frame_end: &mut Option<Box<dyn GpuFuture>>) {
        match self.render_stage {
            RenderStage::Directional => {}
            RenderStage::Point => {}
            RenderStage::RedrawNeeded => {
                self.recreate_swapchain();
                self.commands = None;
                self.render_stage = RenderStage::Stopped;
                return;
            }
            _ => {
                self.commands = None;
                self.render_stage = RenderStage::Stopped;
                return;
            }
        }

        let mut commands = self.commands.take().unwrap();
        commands.end_render_pass().unwrap();
        let command_buffer = commands.build().unwrap();

        let acquire_future = self.acquire_future.take().unwrap();

        let mut local_future: Option<Box<dyn GpuFuture>> =
            Some(Box::new(vulkano::sync::now(self.device.clone())) as Box<dyn GpuFuture>);

        std::mem::swap(&mut local_future, previous_frame_end);

        let future = local_future
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(
                    self.swapchain.clone(),
                    self.image_index,
                ),
            )
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                *previous_frame_end = Some(Box::new(future) as Box<_>);
            }
            Err(vulkano::sync::FlushError::OutOfDate) => {
                self.recreate_swapchain();
                *previous_frame_end =
                    Some(Box::new(vulkano::sync::now(self.device.clone())) as Box<_>);
            }
            Err(err) => {
                println!("Failed to flush future: {:?}", err);
                *previous_frame_end =
                    Some(Box::new(vulkano::sync::now(self.device.clone())) as Box<_>);
            }
        }

        self.commands = None;
        self.render_stage = RenderStage::Stopped;
    }

    fn window_size_dependent_setup(
        memory_allocator: &StandardMemoryAllocator,
        images: &[Arc<SwapchainImage>],
        render_pass: Arc<RenderPass>,
        viewport: &mut Viewport,
    ) -> (Vec<Arc<Framebuffer>>, Arc<ImageView<AttachmentImage>>) {
        let dimensions = images[0].dimensions().width_height();
        viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

        let depth_buffer = ImageView::new_default(
            AttachmentImage::transient(memory_allocator, dimensions, Format::D16_UNORM).unwrap(),
        )
        .unwrap();

        let color_buffer = ImageView::new_default(
            AttachmentImage::transient_input_attachment(
                memory_allocator,
                dimensions,
                Format::A2B10G10R10_UNORM_PACK32,
            )
            .unwrap(),
        )
        .unwrap();

        let framebuffers = images
            .iter()
            .map(|image| {
                let view = ImageView::new_default(image.clone()).unwrap();
                Framebuffer::new(
                    render_pass.clone(),
                    FramebufferCreateInfo {
                        attachments: vec![view, color_buffer.clone(), depth_buffer.clone()],
                        ..Default::default()
                    },
                )
                .unwrap()
            })
            .collect::<Vec<_>>();

        (framebuffers, color_buffer.clone())
    }
}
