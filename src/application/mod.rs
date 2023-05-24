mod renderer;
use renderer::renderables::lights::{DirectionalLight, PointLight};
use renderer::renderables::quad::ColorQuad;
use renderer::renderables::triangle::ColorTriangle;
use renderer::renderables::vertices::ColorVertex2D;
use renderer::renderables::ColorRenderable;
use renderer::Renderer;

use vulkano::sync::{self, GpuFuture};

use nalgebra_glm::vec3;

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

        let time_start = std::time::Instant::now();

        // just for testing
        let mut quad = ColorQuad {
            vertices: [
                ColorVertex2D {
                    position: [-1.0, -1.0, -1.6],
                    color: [1.0, 0.0, 0.0]
                },
                ColorVertex2D {
                    position: [-1.0, 1.0, -1.6],
                    color: [0.0, 1.0, 0.0]
                },
                ColorVertex2D {
                    position: [1.0, 1.0, -1.6],
                    color: [0.0, 0.0, 1.0]
                },
                ColorVertex2D {
                    position: [-1.0, -1.0, -1.6],
                    color: [1.0, 0.0, 0.0]
                },
                ColorVertex2D {
                    position: [1.0, 1.0, -1.6],
                    color: [0.0, 0.0, 1.0]
                },
                ColorVertex2D {
                    position: [1.0, -1.0,-1.6],
                    color: [0.0, 1.0, 0.0]
                }
            ],
            translation: nalgebra_glm::identity(),
            rotation: nalgebra_glm::identity()
        };

        let mut triangle = ColorTriangle {
            vertices: [
                ColorVertex2D {
                    position: [-0.5, 0.5, -1.0],
                    color: [1.0, 0.0, 0.0],
                },
                ColorVertex2D {
                    position: [0.5, 0.5, -1.0],
                    color: [0.0, 1.0, 0.0],
                },
                ColorVertex2D {
                    position: [0.0, -0.5, -1.0],
                    color: [0.0, 0.0, 1.0],
                },
            ],
            translation: nalgebra_glm::identity(),
            rotation: nalgebra_glm::identity()
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

        self.renderer.set_view(
            &nalgebra_glm::look_at(
                &nalgebra_glm::vec3(0.0, 0.0, 0.1),
                &nalgebra_glm::vec3(0.0, 0.0, 0.0),
                &nalgebra_glm::vec3(0.0, 1.0, 0.0),
            )
        );

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

                    let elapsed = time_start.elapsed().as_secs() as f64
                        + time_start.elapsed().subsec_nanos() as f64
                        / 1_000_000_000.0;
                    let rads = elapsed * nalgebra_glm::pi::<f64>() / 180.0;

                    triangle.reset_rotation();
                    triangle.rotate(
                        rads as f32 * 50.0, 
                        vec3(0.0, 0.0, 1.0)
                    );

                    quad.reset_rotation();
                    quad.rotate(
                        rads as f32 * 10.0,
                        vec3(0.0, 0.0, 1.0)
                    );

                    self.renderer.start();
                    self.renderer.color_draw(&quad);
                    self.renderer.color_draw(&triangle);
                    self.renderer.ambient();
                    self.renderer.directional(&dir_light);
                    self.renderer.point(&point_light);
                    self.renderer.render(&mut previous_frame_end);
                }
                _ => {}
            });
    }
}
