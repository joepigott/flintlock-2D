mod renderer;
use renderer::Renderer;

use winit::event_loop::{EventLoop, ControlFlow};
use winit::event::{Event, WindowEvent};

pub struct Application {
    event_loop: EventLoop<()>,
    renderer: Renderer
}

impl Application {
    pub fn new() -> Application {
        let event_loop = EventLoop::new();
        let renderer = Renderer::new(&event_loop);

        Application {
            event_loop,
            renderer
        }
    }

    pub fn run(mut self) {
        self.event_loop.run(move |event, _, control_flow| {
            match event {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                    *control_flow = ControlFlow::Exit;
                }
                Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
                    self.renderer.recreate_swapchain();
                }
                Event::RedrawEventsCleared => {
                    println!("cleared!");
                }
                _ => {}
            }
        });
    }
}
