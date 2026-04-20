mod app;
mod camera;
mod chunk;
mod controller;
mod gpu_context;
mod mesh;
mod state;
mod texture;
mod world;

fn main() {
    app::run().unwrap();
}