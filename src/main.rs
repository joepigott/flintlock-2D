#![allow(non_snake_case)]

mod application;
use application::Application;

fn main() {
    let app = Application::new();
    app.run();
}
