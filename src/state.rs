use winit::window::Window;
use std::sync::Arc;

pub struct State {
    window: Arc<Window>
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<State>{
        Ok(Self {
            window
        })
    }
}