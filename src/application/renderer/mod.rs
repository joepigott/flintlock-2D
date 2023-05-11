#![allow(dead_code, unused)]

use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, CpuBufferPool};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::device::{
    Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags,
};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{AttachmentImage, ImageAccess, SwapchainImage};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::memory::allocator::MemoryUsage;
use vulkano::memory::allocator::{AllocationCreateInfo, StandardMemoryAllocator};
use vulkano::pipeline::graphics::color_blend::{
    AttachmentBlend, BlendFactor, BlendOp, ColorBlendState,
};
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::rasterization::{CullMode, RasterizationState};
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::GraphicsPipeline;
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::swapchain::{
    Surface, Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo, SwapchainCreationError,
};
use vulkano::VulkanLibrary;

use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use vulkano_win::{required_extensions, VkSurfaceBuild};

use bytemuck::{Pod, Zeroable};

use std::io::Cursor;
use std::sync::Arc;

mod shaders;
use shaders::*;

pub struct Renderer {
    surface: Arc<Surface>,
    pub device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    descriptor_set_allocator: StandardDescriptorSetAllocator,
    command_buffer_allocator: StandardCommandBufferAllocator,
    render_pass: Arc<RenderPass>,
    deferred_pipeline: PipelineInfo,
    ambient_pipeline: PipelineInfo,
    point_pipeline: PipelineInfo,
    directional_pipeline: PipelineInfo,
    model_uniform_buffer: CpuBufferPool<deferred_vert::ty::ModelData>,
    ambient_buffer: Arc<CpuAccessibleBuffer<ambient_frag::ty::AmbientData>>,
    point_buffer: CpuBufferPool<point_frag::ty::PointData>,
    directional_buffer: CpuBufferPool<directional_frag::ty::DirectionalData>,
    screen_vertices: Arc<CpuAccessibleBuffer<[BasicVertex2D]>>,
    viewport: Viewport,
    framebuffers: Vec<Arc<Framebuffer>>,
    color_buffer: Arc<ImageView<AttachmentImage>>,
    commands: Option<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>>,
    image_index: u32,
    acquire_future: Option<SwapchainAcquireFuture>,
    render_stage: RenderStage,
}

impl Renderer {
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
            swapchain,
            memory_allocator,
            descriptor_set_allocator,
            command_buffer_allocator,
            render_pass,
            deferred_pipeline,
            ambient_pipeline,
            point_pipeline,
            directional_pipeline,
            model_uniform_buffer,
            ambient_buffer,
            point_buffer,
            directional_buffer,
            screen_vertices,
            viewport,
            framebuffers,
            color_buffer,
            commands,
            image_index,
            acquire_future,
            render_stage,
        }
    }

    pub fn recreate_swapchain(&mut self) {
        let window = self
            .surface
            .object()
            .unwrap()
            .downcast_ref::<Window>()
            .unwrap();
        let image_extent: [u32; 2] = window.inner_size().into();

        let aspect_ratio = image_extent[0] as f32 / image_extent[1] as f32;

        let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
            image_extent,
            ..self.swapchain.create_info()
        }) {
            Ok(r) => r,
            Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
            Err(err) => panic!("Failed to recreate swapchain: {:?}", err),
        };

        self.swapchain = new_swapchain;
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

// render stage allows renderer to function as state machine
enum RenderStage {
    Stopped,
    Vertex,
    Ambient,
    Point,
    Directional,
    RedrawNeeded,
}

// vertex structs are used to define data contained in buffers

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
struct Vertex2D {
    position: [f32; 3],
    uv: [f32; 2],
}
vulkano::impl_vertex!(Vertex2D, position, uv);

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
struct BasicVertex2D {
    position: [f32; 2],
}
vulkano::impl_vertex!(BasicVertex2D, position);

impl BasicVertex2D {
    pub fn screen_vertices() -> [BasicVertex2D; 6] {
        [
            BasicVertex2D {
                position: [-1.0, -1.0],
            },
            BasicVertex2D {
                position: [-1.0, 1.0],
            },
            BasicVertex2D {
                position: [1.0, 1.0],
            },
            BasicVertex2D {
                position: [-1.0, -1.0],
            },
            BasicVertex2D {
                position: [1.0, 1.0],
            },
            BasicVertex2D {
                position: [1.0, -1.0],
            },
        ]
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
struct ColorVertex2D {
    position: [f32; 3],
    color: [f32; 3],
}
vulkano::impl_vertex!(ColorVertex2D, position, color);
