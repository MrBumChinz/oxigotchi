mod config;
mod display;
mod epoch;
mod personality;
mod wifi;
mod attacks;
mod capture;
mod web;
mod bluetooth;
mod pisugar;
mod recovery;
mod migration;

use log::info;

fn main() {
    env_logger::init();
    info!("Rusty Oxigotchi v{} starting — the bull is awake", env!("CARGO_PKG_VERSION"));

    let config = config::Config::load_or_default("/etc/pwnagotchi/config.toml");
    info!("name: {}", config.name);

    let mut screen = display::Screen::new(config.display.clone());
    screen.clear();
    screen.draw_face(&personality::Face::Awake);
    screen.draw_name(&config.name);
    screen.draw_status("Booting...");
    screen.flush();

    info!("display initialized");

    // TODO: main epoch loop integration
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
