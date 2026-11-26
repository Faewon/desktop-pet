mod context;
mod pet;
mod sprite;
mod wayland;

use std::env;
use std::path::PathBuf;

use sprite::SpriteSheet;

fn main() {
    let custom_sprite = env::args()
        .skip(1)
        .collect::<Vec<_>>()
        .windows(2)
        .find(|w| w[0] == "--sprite")
        .map(|w| PathBuf::from(&w[1]));

    let sheet = match SpriteSheet::load(custom_sprite.as_deref()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to load sprite sheet: {e}");
            std::process::exit(1);
        }
    };

    let context = context::spawn_context_watcher();

    // Give the initial Hyprland monitor/window poll a brief moment to populate screen dimensions
    std::thread::sleep(std::time::Duration::from_millis(120));

    println!("desktop-pet running on Hyprland/Wayland!");
    println!("Controls: Left-click = pet | Drag = move/perch on windows | Right-click = cycle mode | Middle-click = quit");

    if let Err(e) = wayland::run_overlay(sheet, context) {
        eprintln!("Overlay error: {e}");
        std::process::exit(1);
    }
}
