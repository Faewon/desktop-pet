# desktop-pet (`Futaba Sakura`)

A lightweight pixel-art desktop pet for **Hyprland / Wayland** written in Rust. Futaba roams your screen, hops onto and perches on open windows, hacks as **The Chair Gremlin**, backseat-games when you play, and reacts in real time to your desktop activity.

## Features

- **Native Wayland `wlr-layer-shell` overlay (`Layer::Overlay`)**: Transparent, visible even over borderless fullscreen games, click-through everywhere except on Futaba (`wl_region`), and never steals keyboard focus.
- **Window perching, throwing & superhero landing**: Reads live window coordinates from Hyprland IPC so she can jump onto open windows, curl into a panicked ball when dragged, launch with flick-proportional throw velocity (with wall/ceiling bounces), flail while falling (`fall_0`–`fall_1`), and strike a 2-stage superhero landing (`land_0`–`land_1`).
- **Futaba Signature Animations** (built-in 41-frame `68×68` sheet at `assets/futaba.png`):
  - **The Chair Gremlin** (`8 FPS`): Squatting in her office chair at a glowing laptop on a two-legged desk, with green screen light, headphone bounce/LED pulse, neon-green glasses flashes, and floating `CodeBit` particles.
  - **Backseat Co-Pilot** (`gamer_0`–`gamer_3`): When gaming (Steam, Gamescope, Lutris, Heroic, emulators, etc.), pulls out a controller, mashes buttons, and drops navigator callouts while staying parked out of your crosshair.
  - **Yakisoba Slurp Fidget**: Every ~40–50s (or during YouTube/video playback) she pulls out instant yakisoba noodles and slurps them.
  - **Caught Peeking**: Hovering your cursor closely while she's hacking triggers a `!` alert and a 3-stage Freeze → Hide → Slow Peek reaction behind her chair.
- **Multi-line typewriter speech bubbles**: Canonical *Persona 5* Oracle lines wrap cleanly across up to 4 lines and animate with a typewriter reveal.
- **Real-time screen & media reactions** (via Hyprland `.socket2.sock`, `hyprctl`, and `playerctl`):
  - Distinguishes **Spotify/music** (`Vibe` with music notes) from **YouTube/video** (`Snack`/`Sit`), reacts to coding, chatting, gaming, and sleeps when idle >45s (except during games).
- **Custom sprite sheets**: Pass `--sprite path/to/sheet.png` (with an optional matching `.json` sidecar; `assets/sprites.png` is also included).

## Controls

- **Left-click**: Pet her (headpats + hearts!)
- **Left-drag / Flick**: Pick her up, drop her on windows, or toss her across the screen
- **Right-click**: Cycle modes (`Nav: Auto Support!` → `Nav: Follow Cursor!` → `Nav: Camp Here`)
- **Double middle-click** (within 2s): Log out / quit

## Build & Run

```bash
cargo build --release
./target/release/desktop-pet
```

### Autostart & Hotkey in Hyprland (`user.lua`)

```lua
hl.on("hyprland.start", function()
    hl.exec_cmd("sleep 1 && (pgrep -x desktop-pet >/dev/null || /path/to/desktop-pet/target/release/desktop-pet)")
end)
hl.bind("SUPER + SHIFT + F", hl.dsp.exec_cmd("pkill -x desktop-pet || /path/to/desktop-pet/target/release/desktop-pet"))
```
