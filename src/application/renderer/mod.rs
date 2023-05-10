#![allow(dead_code, unused)]

use vulkano::VulkanLibrary;
use vulkano::swapchain::{Swapchain, SwapchainCreateInfo, SwapchainCreationError, Surface};
use vulkano::device::{Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags};
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::format::Format;
use vulkano::render_pass::{RenderPass, Subpass};
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::graphics::vertex_input::Vertex;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::viewport::ViewportState;
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::rasterization::{RasterizationState, CullMode};
use vulkano::pipeline::graphics::color_blend::{ColorBlendState, AttachmentBlend, BlendOp, BlendFactor};
use vulkano::buffer::BufferContents;
use vulkano::image::{ImmutableImage, ImageDimensions};
use vulkano::image::view::ImageView;

use winit::window::{Window, WindowBuilder};
use winit::event_loop::EventLoop;

use vulkano_win::{VkSurfaceBuild, required_extensions};

use std::sync::Arc;
use std::io::Cursor;

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
    deferred_pipeline: Arc<GraphicsPipeline>,
    ambient_pipeline: Arc<GraphicsPipeline>,
    point_light_pipeline: Arc<GraphicsPipeline>,
    directional_light_pipeline: Arc<GraphicsPipeline>
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
                }
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

        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()
            .unwrap()
            .filter(|device| device.supported_extensions().contains(&device_extensions))
            .filter_map(|device| {
                device.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        // pick the first queue index that can handle graphics
                        q.queue_flags.contains(QueueFlags::GRAPHICS)
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
                    _ => 5
                }
            })
            .expect("No suitable GPU found.");

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: device_extensions,
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                ..Default::default()
            }
        )
        .unwrap();

        let queue = queues.next().unwrap();

        let (mut swapchain, images) = {
            let caps = device
                .physical_device()
                .surface_capabilities(&surface, Default::default())
                .unwrap();

            let image_usage = caps.supported_usage_flags;
            let composite_alpha = caps.supported_composite_alpha.into_iter().next().unwrap();

            let image_format = Some(
                device
                    .physical_device()
                    .surface_formats(&surface, Default::default())
                    .unwrap()[0]
                    .0
            );

            let window = surface.object().unwrap().downcast_ref::<Window>().unwrap();
            let image_extent: [u32; 2] = window.inner_size().into();

            Swapchain::new(
                device.clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: caps.min_image_count,
                    image_usage,
                    composite_alpha,
                    image_format,
                    image_extent,
                    ..Default::default()
                }
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
        let point_light_vert = point_light_vert::load(device.clone()).unwrap();
        let point_light_frag = point_light_frag::load(device.clone()).unwrap();
        let directional_light_vert = directional_light_vert::load(device.clone()).unwrap();
        let directional_light_frag = directional_light_frag::load(device.clone()).unwrap();

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
            .vertex_input_state(Vertex2D::per_vertex())
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
            .vertex_input_state(BasicVertex2D::per_vertex())
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
                        alpha_destination: BlendFactor::One
                    }
                )
            )
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
            .render_pass(lighting_pass.clone())
            .build(device.clone())
            .unwrap();

        let point_light_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BasicVertex2D::per_vertex())
            .vertex_shader(point_light_vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(point_light_frag.entry_point("main").unwrap(), ())
            .color_blend_state(
                ColorBlendState::new(lighting_pass.num_color_attachments()).blend(
                    AttachmentBlend {
                        color_op: BlendOp::Add,
                        color_source: BlendFactor::One,
                        color_destination: BlendFactor::One,
                        alpha_op: BlendOp::Add,
                        alpha_source: BlendFactor::One,
                        alpha_destination: BlendFactor::One
                    }
                )
            )
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
            .render_pass(lighting_pass.clone())
            .build(device.clone())
            .unwrap();

        let directional_light_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BasicVertex2D::per_vertex())
            .vertex_shader(directional_light_vert.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(directional_light_frag.entry_point("main").unwrap(), ())
            .color_blend_state(
                ColorBlendState::new(lighting_pass.num_color_attachments()).blend(
                    AttachmentBlend {
                        color_op: BlendOp::Add,
                        color_source: BlendFactor::One,
                        color_destination: BlendFactor::One,
                        alpha_op: BlendOp::Add,
                        alpha_source: BlendFactor::One,
                        alpha_destination: BlendFactor::One
                    }
                )
            )
            .rasterization_state(RasterizationState::new().cull_mode(CullMode::Back))
            .render_pass(lighting_pass.clone())
            .build(device.clone())
            .unwrap();

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
            point_light_pipeline,
            directional_light_pipeline
        }
    }

    pub fn recreate_swapchain(&mut self) {
        let window = self.surface
            .object()
            .unwrap()
            .downcast_ref::<Window>()
            .unwrap();
        let image_extent: [u32; 2] = window.inner_size().into();

        let aspect_ratio = image_extent[0] as f32 / image_extent[1] as f32;

        let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo{
            image_extent,
            ..self.swapchain.create_info()
        }) {
            Ok(r) => r,
            Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
            Err(err) => panic!("Failed to recreate swapchain: {:?}", err)
        };

        self.swapchain = new_swapchain;
    }
}

#[repr(C)]
#[derive(BufferContents, Vertex)]
struct Vertex2D {
    #[format(R32G32B32_SFLOAT)]
    position: [f32; 3],
    #[format(R32G32_SFLOAT)]
    uv: [f32; 2]
}

#[repr(C)]
#[derive(BufferContents, Vertex)]
struct BasicVertex2D {
    #[format(R32G32_SFLOAT)]
    position: [f32; 2]
}
