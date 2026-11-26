use std::sync::{Arc, Mutex};

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, Region},
    delegate_compositor, delegate_layer, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
    shm::{slot::SlotPool, Shm, ShmHandler},
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
    Connection, QueueHandle,
};

use crate::context::ScreenContext;
use crate::pet::Pet;
use crate::sprite::{Canvas, SpriteSheet, CANVAS_H, CANVAS_W, PIXEL_SCALE};

pub fn run_overlay(
    sheet: SpriteSheet,
    context: Arc<Mutex<ScreenContext>>,
) -> Result<(), String> {
    let conn = Connection::connect_to_env().map_err(|e| format!("Wayland connect error: {e}"))?;
    let (globals, mut event_queue) =
        registry_queue_init(&conn).map_err(|e| format!("Wayland registry error: {e}"))?;
    let qh = event_queue.handle();

    let compositor =
        CompositorState::bind(&globals, &qh).map_err(|e| format!("wl_compositor bind: {e}"))?;
    let layer_shell =
        LayerShell::bind(&globals, &qh).map_err(|e| format!("wlr_layer_shell bind: {e}"))?;
    let shm = Shm::bind(&globals, &qh).map_err(|e| format!("wl_shm bind: {e}"))?;

    let surface = compositor.create_surface(&qh);
    let layer = layer_shell.create_layer_surface(
        &qh,
        surface,
        Layer::Overlay,
        Some("desktop-pet"),
        None,
    );
    layer.set_anchor(Anchor::TOP | Anchor::LEFT);
    layer.set_size(CANVAS_W, CANVAS_H);
    layer.set_exclusive_zone(-1);
    layer.set_keyboard_interactivity(KeyboardInteractivity::None);
    layer.commit();

    let pool = SlotPool::new((CANVAS_W * CANVAS_H * 4 * 4) as usize, &shm)
        .map_err(|e| format!("SlotPool error: {e}"))?;

    let initial_ctx = context.lock().unwrap().clone();
    let pet = Pet::new(initial_ctx.screen_w, initial_ctx.screen_h);

    let mut app = PetApp {
        registry_state: RegistryState::new(&globals),
        compositor,
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        shm,
        pool,
        layer,
        pointers: Vec::new(),

        exit: false,
        first_configure: true,
        sheet,
        context,
        pet,

        left_down: false,
        dragging: false,
        drag_start_cursor: (0, 0),
        drag_start_pet: (0.0, 0.0),
        throw_vx: 0.0,
        throw_vy: 0.0,
        last_drag_sample: (0.0, 0.0, std::time::Instant::now()),
        last_drag_move: std::time::Instant::now(),
        last_middle_click: None,
        last_input_rect: None,
    };

    loop {
        event_queue
            .blocking_dispatch(&mut app)
            .map_err(|e| format!("Wayland dispatch error: {e}"))?;
        if app.exit {
            break;
        }
    }

    Ok(())
}

struct PetApp {
    registry_state: RegistryState,
    compositor: CompositorState,
    seat_state: SeatState,
    output_state: OutputState,
    shm: Shm,
    pool: SlotPool,
    layer: LayerSurface,
    pointers: Vec<wl_pointer::WlPointer>,

    exit: bool,
    first_configure: bool,
    sheet: SpriteSheet,
    context: Arc<Mutex<ScreenContext>>,
    pet: Pet,

    left_down: bool,
    dragging: bool,
    drag_start_cursor: (i32, i32),
    drag_start_pet: (f32, f32),
    throw_vx: f32,
    throw_vy: f32,
    last_drag_sample: (f32, f32, std::time::Instant),
    last_drag_move: std::time::Instant,
    last_middle_click: Option<std::time::Instant>,
    last_input_rect: Option<(i32, i32, i32, i32)>,
}

impl PetApp {
    fn update_input_region(&mut self, rect: (i32, i32, i32, i32)) {
        if self.last_input_rect == Some(rect) {
            return;
        }
        if let Ok(region) = Region::new(&self.compositor) {
            region.add(rect.0, rect.1, rect.2, rect.3);
            self.layer.wl_surface().set_input_region(Some(region.wl_region()));
            self.last_input_rect = Some(rect);
        }
    }

    fn begin_drag_tracking(&mut self) {
        let now = std::time::Instant::now();
        self.throw_vx = 0.0;
        self.throw_vy = 0.0;
        self.last_drag_sample = (self.pet.x, self.pet.y, now);
        self.last_drag_move = now;
        self.pet.start_drag();
    }

    fn release_drag_velocity(&self) -> (f32, f32) {
        if self.last_drag_move.elapsed().as_secs_f32() > 0.11 {
            return (0.0, 0.0);
        }
        let vx = if self.throw_vx.abs() < 95.0 { 0.0 } else { self.throw_vx };
        let vy = if self.throw_vy.abs() < 95.0 { 0.0 } else { self.throw_vy };
        (vx, vy)
    }

    fn draw(&mut self, qh: &QueueHandle<Self>) {
        let ctx = self.context.lock().unwrap().clone();

        if self.left_down {
            let dx = ctx.cursor_x - self.drag_start_cursor.0;
            let dy = ctx.cursor_y - self.drag_start_cursor.1;
            if !self.dragging && (dx.abs() > 6 || dy.abs() > 6) {
                self.dragging = true;
                self.begin_drag_tracking();
            }
            if self.dragging {
                let next_x = (self.drag_start_pet.0 + dx as f32)
                    .clamp(36.0, (ctx.screen_w - 36).max(40) as f32);
                let min_y = (ctx.reserved_top + 90) as f32;
                let max_y = (ctx.screen_h as f32).max(min_y + 1.0);
                let next_y = (self.drag_start_pet.1 + dy as f32).clamp(min_y, max_y);

                let now = std::time::Instant::now();
                let moved_x = next_x - self.last_drag_sample.0;
                let moved_y = next_y - self.last_drag_sample.1;
                if moved_x.abs() > 0.5 || moved_y.abs() > 0.5 {
                    let dt_move = now.duration_since(self.last_drag_move).as_secs_f32().max(0.012);
                    let inst_vx = ((moved_x / dt_move) * 0.90).clamp(-1450.0, 1450.0);
                    let inst_vy = ((moved_y / dt_move) * 0.90).clamp(-1150.0, 1150.0);
                    self.throw_vx = self.throw_vx * 0.35 + inst_vx * 0.65;
                    self.throw_vy = self.throw_vy * 0.35 + inst_vy * 0.65;
                    self.last_drag_sample = (next_x, next_y, now);
                    self.last_drag_move = now;
                } else if now.duration_since(self.last_drag_move).as_secs_f32() > 0.085 {
                    self.throw_vx *= 0.45;
                    self.throw_vy *= 0.45;
                }

                self.pet.x = next_x;
                self.pet.y = next_y;
            }
        }

        self.pet.update(&ctx);

        let max_cx = (ctx.screen_w - CANVAS_W as i32).max(0);
        let max_cy = (ctx.screen_h - CANVAS_H as i32).max(0);
        let canvas_x = (self.pet.x.round() as i32 - CANVAS_W as i32 / 2).clamp(0, max_cx);
        let canvas_y = (self.pet.y.round() as i32 - (CANVAS_H as i32 - 8)).clamp(0, max_cy);

        let local_x = (self.pet.x.round() as i32 - canvas_x).clamp(24, CANVAS_W as i32 - 24);
        let local_y = (self.pet.y.round() as i32 - canvas_y).clamp(40, CANVAS_H as i32 - 2);

        self.layer.set_margin(canvas_y, 0, 0, canvas_x);

        // Tight click-through region: only the pet sprite receives pointer events
        let input_rect = if self.left_down || self.dragging {
            (0, 0, CANVAS_W as i32, CANVAS_H as i32)
        } else {
            let hit_w = (self.sheet.frame_w as i32 * PIXEL_SCALE).min(110);
            let hit_h = (self.sheet.frame_h as i32 * PIXEL_SCALE).min(130);
            let hx = (local_x - hit_w / 2).max(0);
            let hy = (local_y - hit_h).max(0);
            (hx, hy, hit_w, hit_h)
        };
        self.update_input_region(input_rect);

        let w = CANVAS_W as i32;
        let h = CANVAS_H as i32;
        let stride = w * 4;
        let wl_surface = self.layer.wl_surface();
        wl_surface.frame(qh, wl_surface.clone());

        let Ok((buffer, raw)) = self
            .pool
            .create_buffer(w, h, stride, wl_shm::Format::Argb8888)
        else {
            self.layer.commit();
            return;
        };

        {
            let mut canvas = Canvas {
                data: raw,
                width: w,
                height: h,
            };
            self.pet.render(&mut canvas, &self.sheet, local_x, local_y);
        }

        wl_surface.damage_buffer(0, 0, w, h);
        if buffer.attach_to(wl_surface).is_ok() {
            self.layer.commit();
        }
    }
}

impl CompositorHandler for PetApp {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.draw(qh);
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for PetApp {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl LayerShellHandler for PetApp {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        _configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        if self.first_configure {
            self.first_configure = false;
            self.draw(qh);
        }
    }
}

impl SeatHandler for PetApp {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer {
            if let Ok(ptr) = self.seat_state.get_pointer(qh, &seat) {
                self.pointers.push(ptr);
            }
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer {
            for ptr in self.pointers.drain(..) {
                ptr.release();
            }
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl PointerHandler for PetApp {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        use PointerEventKind::*;
        const BTN_LEFT: u32 = 0x110;
        const BTN_RIGHT: u32 = 0x111;
        const BTN_MIDDLE: u32 = 0x112;

        for event in events {
            if &event.surface != self.layer.wl_surface() {
                continue;
            }
            match event.kind {
                Press { button: BTN_LEFT, .. } => {
                    let ctx = self.context.lock().unwrap();
                    self.left_down = true;
                    self.dragging = false;
                    self.drag_start_cursor = (ctx.cursor_x, ctx.cursor_y);
                    self.drag_start_pet = (self.pet.x, self.pet.y);
                    drop(ctx);
                    self.update_input_region((0, 0, CANVAS_W as i32, CANVAS_H as i32));
                    self.layer.commit();
                }
                Motion { .. } => {
                    if self.left_down && !self.dragging {
                        let ctx = self.context.lock().unwrap();
                        let dx = ctx.cursor_x - self.drag_start_cursor.0;
                        let dy = ctx.cursor_y - self.drag_start_cursor.1;
                        if dx.abs() > 6 || dy.abs() > 6 {
                            self.dragging = true;
                            drop(ctx);
                            self.begin_drag_tracking();
                        }
                    }
                }
                Release { button: BTN_LEFT, .. } => {
                    if self.left_down {
                        let was_dragging = self.dragging;
                        self.left_down = false;
                        self.dragging = false;
                        if was_dragging {
                            let (tvx, tvy) = self.release_drag_velocity();
                            let ctx = self.context.lock().unwrap().clone();
                            self.pet.end_drag(&ctx, tvx, tvy);
                        } else {
                            self.pet.on_petted();
                        }
                    }
                }
                Leave { .. } => {
                    if self.left_down || self.dragging {
                        let was_dragging = self.dragging;
                        self.left_down = false;
                        self.dragging = false;
                        if was_dragging {
                            let (tvx, tvy) = self.release_drag_velocity();
                            let ctx = self.context.lock().unwrap().clone();
                            self.pet.end_drag(&ctx, tvx, tvy);
                        }
                    }
                }
                Press { button: BTN_RIGHT, .. } => {
                    self.pet.on_right_click();
                }
                Press { button: BTN_MIDDLE, .. } => {
                    let now = std::time::Instant::now();
                    if self
                        .last_middle_click
                        .is_some_and(|t| now.duration_since(t).as_secs_f32() < 2.0)
                    {
                        self.exit = true;
                    } else {
                        self.last_middle_click = Some(now);
                        self.pet.say("Middle-click again to log out!", 2.0, true);
                    }
                }
                _ => {}
            }
        }
    }
}

impl ShmHandler for PetApp {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_compositor!(PetApp);
delegate_output!(PetApp);
delegate_shm!(PetApp);
delegate_seat!(PetApp);
delegate_pointer!(PetApp);
delegate_layer!(PetApp);
delegate_registry!(PetApp);

impl ProvidesRegistryState for PetApp {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}
