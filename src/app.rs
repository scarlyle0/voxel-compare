use crate::state::State;
use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
    window::WindowId
};

pub struct App {
    state: Option<State>
}

impl App {
    pub fn new() -> Self {
        Self {
            state: None,
        }
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Need to use Arc here since surface references window internally
        let window = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());
        // Pass the window to state
        self.state = Some(pollster::block_on(State::new(window)).unwrap());
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let state = match &mut self.state {
            Some(state) => state,
            None => return,
        };
        match event {
            WindowEvent::CloseRequested => {println!("The close button was pressed; stopping"); event_loop.exit();},
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("{e}");
                        event_loop.exit();
                    }
                }
            }
            _ => (),
        }
    }
}

pub fn run() -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = App::new(); // Create empty container state
    event_loop.run_app(&mut app)?; // Start loop call ApplicationHandler
    // -> Calls resumed to initiate window, state 
    // -> Then repeatedly calls window_event until closed
    Ok(())
}