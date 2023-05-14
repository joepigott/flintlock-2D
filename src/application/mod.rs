mod renderer;
use renderer::renderables::lights::{DirectionalLight, PointLight};
use renderer::renderables::quad::ColorQuad;
use renderer::renderables::triangle::ColorTriangle;
use renderer::renderables::vertices::ColorVertex2D;
use renderer::Renderer;

use vulkano::sync::{self, GpuFuture};

use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

pub struct Application {
    event_loop: EventLoop<()>,
    renderer: Renderer,
}

impl Application {
    pub fn new() -> Application {
        let event_loop = EventLoop::new();
        let renderer = Renderer::new(&event_loop);

        Application {
            event_loop,
            renderer,
        }
    }

    pub fn run(mut self) {
        let mut previous_frame_end =
            Some(Box::new(sync::now(self.renderer.device.clone())) as Box<dyn GpuFuture>);

        // just for testing
        // let quad = ColorQuad {
        //     vertices: [
        //         ColorVertex2D {
        //             position: [-1.0, -1.0, 1.0],
        //             color: [1.0, 1.0, 1.0]
        //         },
        //         ColorVertex2D {
        //             position: [-1.0, 1.0, 1.0],
        //             color: [1.0, 1.0, 1.0]
        //         },
        //         ColorVertex2D {
        //             position: [1.0, 1.0, 1.0],
        //             color: [1.0, 1.0, 1.0]
        //         },
        //         ColorVertex2D {
        //             position: [-1.0, -1.0, 1.0],
        //             color: [1.0, 1.0, 1.0]
        //         },
        //         ColorVertex2D {
        //             position: [1.0, 1.0, 1.0],
        //             color: [1.0, 1.0, 1.0]
        //         },
        //         ColorVertex2D {
        //             position: [1.0, -1.0, 1.0],
        //             color: [1.0, 1.0, 1.0]
        //         }
        //     ],
        //     matrix: nalgebra_glm::identity()
        // };

        let triangle = ColorTriangle {
            vertices: [
                ColorVertex2D {
                    position: [-0.5, 0.5, 0.0],
                    color: [1.0, 0.0, 0.0],
                },
                ColorVertex2D {
                    position: [0.5, 0.5, 0.0],
                    color: [0.0, 1.0, 0.0],
                },
                ColorVertex2D {
                    position: [0.0, -0.5, 0.0],
                    color: [0.0, 0.0, 1.0],
                },
            ],
            matrix: nalgebra_glm::identity(),
        };

        let dir_light = DirectionalLight {
            direction: [-1.0, 1.0],
            color: [1.0, 1.0, 1.0],
            intensity: 0.5,
        };

        let point_light = PointLight {
            position: [0.0, 0.0, 0.0],
            color: [1.0, 1.0, 1.0],
            intensity: 0.5,
        };

        self.event_loop
            .run(move |event, _, control_flow| match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    *control_flow = ControlFlow::Exit;
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized(_),
                    ..
                } => {
                    self.renderer.recreate_swapchain();
                }
                Event::RedrawEventsCleared => {
                    previous_frame_end
                        .as_mut()
                        .take()
                        .unwrap()
                        .cleanup_finished();

                    self.renderer.start();
                    self.renderer.color_draw(&triangle);
                    self.renderer.ambient();
                    self.renderer.directional(&dir_light);
                    self.renderer.point(&point_light);
                    self.renderer.finish(&mut previous_frame_end);
                }
                _ => {}
            });
    }
}
