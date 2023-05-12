mod renderer;
use renderer::Renderer;

use vulkano::sync::GpuFuture;

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
            Some(Box::new(vulkano::sync::now(self.renderer.device.clone())) as Box<dyn GpuFuture>);

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
                    self.renderer.finish(&mut previous_frame_end);
                }
                _ => {}
            });
    }
}
