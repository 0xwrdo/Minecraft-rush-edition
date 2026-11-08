use mc_client::App;
use winit::event_loop::EventLoop;

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let app = pollster::block_on(App::new(&event_loop));
    app.run(event_loop);
}
