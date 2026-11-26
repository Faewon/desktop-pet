use std::time::Instant;

use rand::Rng;

use crate::context::{ActivityKind, ScreenContext, ScreenEvent, WindowLedge};
use crate::sprite::{AnimFrame, Canvas, Particle, ParticleKind, SpriteSheet, PIXEL_SCALE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetMode {
    Auto,
    FollowCursor,
    Perch,
}

impl PetMode {
    pub fn next(self) -> Self {
        match self {
            PetMode::Auto => PetMode::FollowCursor,
            PetMode::FollowCursor => PetMode::Perch,
            PetMode::Perch => PetMode::Auto,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PetMode::Auto => "Nav: Auto Support!",
            PetMode::FollowCursor => "Nav: Follow Cursor!",
            PetMode::Perch => "Nav: Camp Here",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaughtPhase {
    Freeze,
    Hide,
    SlowPeek,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetState {
    Walk,
    Sit,
    Vibe,
    CodeCheer,
    BackseatGamer,
    ChairGremlin,
    Snack,
    Caught,
    Sleep,
    Happy,
    PeekCursor,
    Dragged,
    Fall,
    Land,
}

pub struct Pet {
    /// Logical center X of the pet's feet.
    pub x: f32,
    /// Logical bottom Y of the pet's feet.
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub facing_left: bool,
    pub state: PetState,
    pub mode: PetMode,

    pub perched_window: Option<String>,
    state_timer: f32,
    anim_timer: f32,
    particle_timer: f32,
    speech_cooldown: f32,
    jump_cooldown: f32,
    fidget_timer: f32,
    caught_cooldown: f32,
    caught_phase: CaughtPhase,

    pub speech_text: Option<String>,
    speech_timer: f32,
    speech_max: f32,

    pub particles: Vec<Particle>,
    last_seen_seq: u64,
    last_activity: ActivityKind,
    last_tick: Instant,
}

impl Pet {
    pub fn new(screen_w: i32, screen_h: i32) -> Self {
        Self {
            x: (screen_w as f32 * 0.72).max(120.0),
            y: screen_h as f32,
            vx: 0.0,
            vy: 0.0,
            facing_left: false,
            state: PetState::ChairGremlin,
            mode: PetMode::Auto,
            perched_window: None,
            state_timer: 6.5,
            anim_timer: 0.0,
            particle_timer: 0.0,
            speech_cooldown: 4.0,
            jump_cooldown: 2.0,
            fidget_timer: 45.0,
            caught_cooldown: 0.0,
            caught_phase: CaughtPhase::Freeze,
            speech_text: Some("Oracle online! All systems green!".to_string()),
            speech_timer: 3.5,
            speech_max: 3.5,
            particles: Vec::new(),
            last_seen_seq: 0,
            last_activity: ActivityKind::General,
            last_tick: Instant::now(),
        }
    }

    pub fn say(&mut self, text: impl Into<String>, duration: f32, force: bool) {
        if !force && self.speech_cooldown > 0.0 {
            return;
        }
        let msg = text.into();
        let typing_time = msg.chars().count() as f32 / 34.0;
        let effective_dur = duration.max(typing_time + 1.6);
        self.speech_text = Some(msg);
        self.speech_timer = effective_dur;
        self.speech_max = effective_dur;
        if !force {
            self.speech_cooldown = 14.0;
        }
    }

    pub fn on_petted(&mut self) {
        self.state = PetState::Happy;
        self.state_timer = 1.6;
        if self.vy == 0.0 {
            self.vy = -190.0;
        }
        self.spawn_burst(ParticleKind::Heart, 5);
        let lines = [
            "Mwehehe~ ♥",
            "Buff: Morale UP! ♥",
            "H-Hey, messing up my hair!",
            "SP restored~ ♥",
            "Save point reached! Comfy~",
        ];
        let idx = rand::thread_rng().gen_range(0..lines.len());
        self.say(lines[idx], 2.4, true);
    }

    pub fn on_right_click(&mut self) {
        self.mode = self.mode.next();
        self.state = PetState::Happy;
        self.state_timer = 0.8;
        self.spawn_burst(ParticleKind::Sparkle, 3);
        self.say(self.mode.label(), 2.4, true);
    }

    pub fn start_drag(&mut self) {
        self.state = PetState::Dragged;
        self.perched_window = None;
        self.vx = 0.0;
        self.vy = 0.0;
        self.say("Gah! Unwanted contact! Put me down!!", 2.2, true);
    }

    fn trigger_landing(&mut self) {
        self.vx = 0.0;
        self.vy = 0.0;
        self.state = PetState::Land;
        self.state_timer = 0.48;
        self.spawn_burst(ParticleKind::Sparkle, 2);
    }

    pub fn end_drag(&mut self, ctx: &ScreenContext, vx: f32, vy: f32) {
        self.state = PetState::Fall;
        self.vx = vx.clamp(-1450.0, 1450.0);
        self.vy = if vy.abs() > 75.0 {
            vy.clamp(-1150.0, 1150.0)
        } else {
            20.0
        };
        if self.vx.abs() > 60.0 {
            self.facing_left = self.vx < 0.0;
        }
        let throw_speed = (self.vx * self.vx + self.vy * self.vy).sqrt();
        if throw_speed > 420.0 {
            self.spawn_burst(ParticleKind::Sparkle, 3);
            self.say("WAAAH!! Too fast!!", 2.0, true);
        }

        // Only snap onto a nearby ledge when dropped gently, not when thrown upward/fast
        if self.vy >= -50.0 && throw_speed < 240.0 {
            if let Some(ledge) = find_landing_ledge(self.x, self.y - 10.0, self.y + 18.0, ctx) {
                self.y = ledge.y as f32;
                self.perched_window = Some(ledge.address.clone());
                self.trigger_landing();
                let short_app: String = ledge.class.chars().take(12).collect();
                self.say(format!("Claimed high ground: {short_app}!"), 2.0, false);
            }
        }
    }

    fn spawn_burst(&mut self, kind: ParticleKind, count: usize) {
        let mut rng = rand::thread_rng();
        for _ in 0..count {
            self.particles.push(Particle {
                kind,
                variant: rng.gen_range(0..4),
                dx: rng.gen_range(-28.0..28.0),
                dy: rng.gen_range(-115.0..-70.0),
                vx: rng.gen_range(-24.0..24.0),
                vy: rng.gen_range(-55.0..-25.0),
                age: 0.0,
                max_age: rng.gen_range(0.9..1.5),
            });
        }
    }

    pub fn update(&mut self, ctx: &ScreenContext) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f32().clamp(0.001, 0.05);
        self.last_tick = now;

        self.anim_timer += dt;
        self.speech_cooldown = (self.speech_cooldown - dt).max(0.0);
        self.jump_cooldown = (self.jump_cooldown - dt).max(0.0);
        self.caught_cooldown = (self.caught_cooldown - dt).max(0.0);
        self.fidget_timer -= dt;

        if self.speech_timer > 0.0 {
            self.speech_timer -= dt;
            if self.speech_timer <= 0.0 {
                self.speech_text = None;
            }
        }

        // Update floating particles
        for p in &mut self.particles {
            p.age += dt;
            p.dx += p.vx * dt;
            p.dy += p.vy * dt;
        }
        self.particles.retain(|p| p.age < p.max_age);

        if self.state == PetState::Dragged {
            return;
        }

        // Keep perched window synced or detach if window disappeared
        let mut active_ledge: Option<WindowLedge> = None;
        if let Some(addr) = &self.perched_window {
            if let Some(ledge) = ctx.ledges.iter().find(|l| &l.address == addr) {
                if ledge.y >= ctx.reserved_top + 115 {
                    self.y = ledge.y as f32;
                    let min_x = (ledge.x + 28) as f32;
                    let max_x = (ledge.x + ledge.width - 28) as f32;
                    if max_x > min_x {
                        self.x = self.x.clamp(min_x, max_x);
                        active_ledge = Some(ledge.clone());
                    }
                }
            }
            if active_ledge.is_none() {
                self.perched_window = None;
                if self.y < ctx.screen_h as f32 - 2.0 {
                    self.state = PetState::Fall;
                }
            }
        }

        // Gravity / airborne movement (jumps, falls & throws)
        let on_floor = (self.y - ctx.screen_h as f32).abs() < 1.5 && self.perched_window.is_none();
        if self.vy != 0.0 || (!on_floor && self.perched_window.is_none()) {
            let prev_y = self.y;
            self.vy += 680.0 * dt;
            // Smooth air drag proportional to air speed
            self.vx *= (1.0 - 0.85 * dt).max(0.0);
            self.x += self.vx * dt;
            self.y += self.vy * dt;

            // Gentle bonk off the top border if tossed upward
            let ceiling_y = (ctx.reserved_top + 90) as f32;
            if self.y < ceiling_y {
                self.y = ceiling_y;
                if self.vy < 0.0 {
                    self.vy = self.vy.abs() * 0.30;
                    self.spawn_burst(ParticleKind::Sparkle, 2);
                }
            }

            // Transition long downward trajectories (e.g. missed jump) into Fall pose
            if self.vy > 80.0 && self.state != PetState::Happy {
                self.state = PetState::Fall;
            }

            // Landing on a window ledge while falling downward
            if self.vy > 0.0 {
                if let Some(ledge) = find_landing_ledge(self.x, prev_y, self.y, ctx) {
                    self.y = ledge.y as f32;
                    self.perched_window = Some(ledge.address.clone());
                    active_ledge = Some(ledge);
                    if self.state == PetState::Fall {
                        self.trigger_landing();
                    } else {
                        self.vy = 0.0;
                        self.vx = 0.0;
                    }
                }
            }

            // Landing on screen floor
            if self.y >= ctx.screen_h as f32 {
                self.y = ctx.screen_h as f32;
                self.perched_window = None;
                if self.state == PetState::Fall {
                    self.trigger_landing();
                } else {
                    self.vy = 0.0;
                }
            }
        }

        let is_grounded = active_ledge.is_some()
            || ((self.y - ctx.screen_h as f32).abs() < 1.5 && self.vy == 0.0);

        // React to instantaneous screen events after physics settles grounded state
        self.process_screen_events(ctx, is_grounded);

        // Sync polled activity changes (e.g. startup with app focused or workspace switch)
        if ctx.activity != self.last_activity {
            let can_interrupt = is_grounded
                && !matches!(
                    self.state,
                    PetState::Dragged | PetState::Fall | PetState::Land | PetState::Happy | PetState::Caught
                );
            self.last_activity = ctx.activity;
            if can_interrupt && self.mode != PetMode::FollowCursor {
                match ctx.activity {
                    ActivityKind::Gaming => {
                        self.state = PetState::BackseatGamer;
                        self.state_timer = 8.0;
                        self.vx = 0.0;
                        self.say("Navigator ready! Don't feed, okay?", 2.8, false);
                    }
                    ActivityKind::Coding => {
                        self.state = PetState::ChairGremlin;
                        self.state_timer = 7.5;
                        self.vx = 0.0;
                    }
                    _ => {}
                }
            }
        }

        // Wake up from sleep if user becomes active
        if is_grounded && self.state == PetState::Sleep && ctx.user_idle_secs < 2.0 {
            self.state = PetState::ChairGremlin;
            self.state_timer = 4.0;
            self.say("Ugh... five more minutes, Sojiro...", 2.2, false);
        }

        // Play out landing recovery across all modes before resuming normal behavior
        if self.state == PetState::Land {
            self.vx = 0.0;
            self.state_timer -= dt;
            if self.state_timer <= 0.0 {
                self.state = match self.mode {
                    PetMode::FollowCursor => PetState::PeekCursor,
                    _ if ctx.activity == ActivityKind::Gaming => {
                        self.state_timer = rand::thread_rng().gen_range(5.0..8.5);
                        PetState::BackseatGamer
                    }
                    _ if self.perched_window.is_some() => {
                        self.state_timer = rand::thread_rng().gen_range(4.0..8.0);
                        PetState::Sit
                    }
                    _ => {
                        self.state_timer = rand::thread_rng().gen_range(3.5..6.5);
                        PetState::ChairGremlin
                    }
                };
            }
        } else if self.state == PetState::Fall || (!is_grounded && self.state != PetState::Happy) {
            // Keep Fall locked in the air so timers, cursor follow, or mode logic cannot interrupt it
            self.state = PetState::Fall;
        } else if is_grounded {
            // Mode-specific grounded behavior
            match self.mode {
                PetMode::FollowCursor => self.update_follow_cursor(ctx, dt),
                PetMode::Perch => self.update_ambient_react(ctx, dt, false),
                PetMode::Auto => self.update_auto(ctx, dt, active_ledge.as_ref()),
            }
        }

        // Clamp / bounce horizontal bounds inside monitor
        let margin = 44.0;
        let max_x = (ctx.screen_w as f32 - margin).max(margin + 10.0);
        if self.x < margin {
            self.x = margin;
            let bounce = if is_grounded { 1.0 } else { 0.55 };
            if !is_grounded && self.vx.abs() > 360.0 {
                self.spawn_burst(ParticleKind::Sparkle, 2);
            }
            self.vx = self.vx.abs() * bounce;
            self.facing_left = false;
        } else if self.x > max_x {
            self.x = max_x;
            let bounce = if is_grounded { 1.0 } else { 0.55 };
            if !is_grounded && self.vx.abs() > 360.0 {
                self.spawn_burst(ParticleKind::Sparkle, 2);
            }
            self.vx = -self.vx.abs() * bounce;
            self.facing_left = true;
        }
    }

    fn process_screen_events(&mut self, ctx: &ScreenContext, is_grounded: bool) {
        if ctx.last_event_seq == self.last_seen_seq {
            return;
        }
        self.last_seen_seq = ctx.last_event_seq;
        let Some(ev) = &ctx.last_event_kind else {
            return;
        };
        let can_interrupt =
            is_grounded && !matches!(self.state, PetState::Dragged | PetState::Fall | PetState::Land);
        match ev {
            ScreenEvent::MusicStarted(track) => {
                if can_interrupt && self.mode != PetMode::Perch {
                    self.state = PetState::Vibe;
                    self.state_timer = 7.0;
                    self.vx = 0.0;
                }
                self.spawn_burst(ParticleKind::MusicNote, 3);
                let short: String = track.chars().take(18).collect();
                self.say(format!("♪ BGM set: {short}"), 3.2, true);
            }
            ScreenEvent::MusicStopped => {
                if can_interrupt && self.state == PetState::Vibe {
                    self.state = PetState::ChairGremlin;
                    self.state_timer = 4.5;
                }
            }
            ScreenEvent::VideoStarted(title) => {
                if can_interrupt && self.mode != PetMode::Perch {
                    self.state = PetState::Snack;
                    self.state_timer = 3.2;
                    self.vx = 0.0;
                }
                let short: String = title.chars().take(16).collect();
                self.say(format!("▶ YT: {short}"), 3.0, true);
            }
            ScreenEvent::VideoStopped => {
                if can_interrupt && matches!(self.state, PetState::Sit | PetState::Snack) {
                    self.state = PetState::ChairGremlin;
                    self.state_timer = 4.5;
                }
            }
            ScreenEvent::WindowChanged { class, title } => {
                let kind = crate::context::classify_activity(
                    class,
                    title,
                    ctx.music_playing,
                    ctx.video_playing,
                    ctx.user_idle_secs,
                );
                if kind != self.last_activity {
                    self.last_activity = kind;
                    match kind {
                        ActivityKind::Coding => {
                            if can_interrupt
                                && self.state != PetState::Happy
                                && self.state != PetState::Caught
                            {
                                self.state = PetState::ChairGremlin;
                                self.state_timer = 7.5;
                                self.vx = 0.0;
                            }
                            self.spawn_burst(ParticleKind::CodeBit, 4);
                            self.say("Alibaba mode! Bypassing firewall~", 2.6, false);
                        }
                        ActivityKind::MediaVideo => {
                            self.say("Is this lore? Wait, no spoilers!", 2.6, false);
                        }
                        ActivityKind::Gaming => {
                            if can_interrupt
                                && self.mode != PetMode::FollowCursor
                                && self.state != PetState::Happy
                                && self.state != PetState::Caught
                            {
                                self.state = PetState::BackseatGamer;
                                self.state_timer = 8.0;
                                self.vx = 0.0;
                            }
                            self.spawn_burst(ParticleKind::Sparkle, 3);
                            self.say("Navigator ready! Don't feed, okay?", 2.8, true);
                        }
                        ActivityKind::Chatting => {
                            self.say("A wild NPC messaged you!", 2.4, false);
                        }
                        _ => {}
                    }
                }
            }
            ScreenEvent::WorkspaceChanged => {
                self.perched_window = None;
                if self.y < ctx.screen_h as f32 {
                    self.state = PetState::Fall;
                }
                self.say("Fast-travel complete!", 1.8, false);
            }
            ScreenEvent::WindowOpened => {
                self.spawn_burst(ParticleKind::Sparkle, 2);
            }
        }
    }

    fn update_follow_cursor(&mut self, ctx: &ScreenContext, dt: f32) {
        let dx = ctx.cursor_x as f32 - self.x;
        let dy = ctx.cursor_y as f32 - (self.y - 50.0);
        self.facing_left = dx < 0.0;

        if dx.abs() > 42.0 {
            self.state = PetState::Walk;
            let speed = 165.0;
            self.vx = dx.signum() * speed;
            self.x += self.vx * dt;
            if dy < -95.0 && self.vy == 0.0 && self.jump_cooldown <= 0.0 {
                self.vy = -420.0;
                self.perched_window = None;
                self.jump_cooldown = 1.4;
            }
        } else {
            self.vx = 0.0;
            self.state = PetState::PeekCursor;
        }
    }

    fn update_ambient_react(&mut self, ctx: &ScreenContext, dt: f32, allow_walk: bool) {
        // Ambient particle emitter for active states
        self.particle_timer -= dt;
        if self.particle_timer <= 0.0 {
            match self.state {
                PetState::Vibe => {
                    self.particle_timer = 0.75;
                    self.spawn_burst(ParticleKind::MusicNote, 1);
                }
                PetState::CodeCheer => {
                    self.particle_timer = 1.1;
                    self.spawn_burst(ParticleKind::Sparkle, 1);
                }
                PetState::BackseatGamer => {
                    self.particle_timer = 0.72;
                    let mut rng = rand::thread_rng();
                    self.particles.push(Particle {
                        kind: if rng.gen_bool(0.45) {
                            ParticleKind::Sparkle
                        } else {
                            ParticleKind::CodeBit
                        },
                        variant: rng.gen_range(0..4),
                        dx: rng.gen_range(-12.0..18.0),
                        dy: rng.gen_range(-62.0..-42.0),
                        vx: rng.gen_range(-12.0..12.0),
                        vy: rng.gen_range(-38.0..-22.0),
                        age: 0.0,
                        max_age: rng.gen_range(0.65..1.1),
                    });
                }
                PetState::ChairGremlin => {
                    self.particle_timer = 0.38;
                    let dir = if self.facing_left { -1.0 } else { 1.0 };
                    let mut rng = rand::thread_rng();
                    self.particles.push(Particle {
                        kind: ParticleKind::CodeBit,
                        variant: rng.gen_range(0..4),
                        dx: dir * rng.gen_range(8.0..34.0),
                        dy: rng.gen_range(-58.0..-36.0),
                        vx: rng.gen_range(-10.0..10.0),
                        vy: rng.gen_range(-42.0..-24.0),
                        age: 0.0,
                        max_age: rng.gen_range(0.75..1.25),
                    });
                }
                PetState::Sleep => {
                    self.particle_timer = 1.4;
                    self.spawn_burst(ParticleKind::Zzz, 1);
                }
                _ => {
                    self.particle_timer = 0.8;
                }
            }
        }

        // Cursor hover / "Caught" reaction while hacking
        let dist_cursor = ((ctx.cursor_x as f32 - self.x).powi(2)
            + (ctx.cursor_y as f32 - (self.y - 60.0)).powi(2))
        .sqrt();
        if matches!(self.state, PetState::ChairGremlin | PetState::CodeCheer)
            && dist_cursor < 52.0
            && self.caught_cooldown <= 0.0
        {
            self.state = PetState::Caught;
            self.caught_phase = CaughtPhase::Freeze;
            self.state_timer = 0.22;
            self.vx = 0.0;
            self.particles.push(Particle {
                kind: ParticleKind::Alert,
                variant: 0,
                dx: -4.0,
                dy: -124.0,
                vx: 0.0,
                vy: -14.0,
                age: 0.0,
                max_age: 0.9,
            });
            self.say("Eek! Don't look at my screen!!", 2.0, true);
            return;
        }

        if self.state == PetState::Caught {
            match self.caught_phase {
                CaughtPhase::Freeze => {
                    self.state_timer -= dt;
                    if self.state_timer <= 0.0 {
                        self.caught_phase = CaughtPhase::Hide;
                        self.state_timer = 0.45;
                    }
                }
                CaughtPhase::Hide => {
                    if dist_cursor < 84.0 {
                        self.state_timer = 0.45;
                    } else {
                        self.state_timer -= dt;
                        if self.state_timer <= 0.0 {
                            self.caught_phase = CaughtPhase::SlowPeek;
                            self.state_timer = 1.05;
                            self.facing_left = (ctx.cursor_x as f32) < self.x;
                            self.say("Are the normies gone...?", 1.2, false);
                        }
                    }
                }
                CaughtPhase::SlowPeek => {
                    if dist_cursor < 64.0 {
                        self.caught_phase = CaughtPhase::Hide;
                        self.state_timer = 0.45;
                    } else {
                        self.state_timer -= dt;
                        if self.state_timer <= 0.0 {
                            self.caught_cooldown = 8.0;
                            self.state = PetState::ChairGremlin;
                            self.state_timer = 6.0;
                        }
                    }
                }
            }
            return;
        }

        // Rare fidget (~every 45s): pull out instant yakisoba/curry and slurp!
        if self.fidget_timer <= 0.0
            && matches!(self.state, PetState::ChairGremlin | PetState::Sit)
        {
            self.fidget_timer = rand::thread_rng().gen_range(40.0..50.0);
            self.state = PetState::Snack;
            self.state_timer = 1.65;
            self.vx = 0.0;
            self.say("*slurp* Sojiro's curry? No, yakisoba~", 1.8, false);
            return;
        }

        if self.state == PetState::Snack {
            self.state_timer -= dt;
            if self.state_timer <= 0.0 {
                self.state = PetState::ChairGremlin;
                self.state_timer = 6.5;
            }
            return;
        }

        if ctx.activity != ActivityKind::Gaming
            && dist_cursor < 95.0
            && ctx.cursor_speed > 1700.0
            && self.vy == 0.0
            && self.jump_cooldown <= 0.0
        {
            self.vy = -210.0;
            self.jump_cooldown = 3.0;
            self.state = PetState::Happy;
            self.state_timer = 1.0;
            self.say("Hey! What kind of APM is that?!", 1.8, false);
            return;
        }

        if self.state == PetState::Happy {
            self.state_timer -= dt;
            if self.state_timer <= 0.0 {
                self.state = if ctx.activity == ActivityKind::Gaming {
                    PetState::BackseatGamer
                } else {
                    PetState::ChairGremlin
                };
                self.state_timer = 4.5;
            }
            return;
        }

        self.state_timer -= dt;
        if self.state_timer > 0.0 {
            return;
        }

        // Pick next reaction/roam state
        let mut rng = rand::thread_rng();
        if ctx.activity == ActivityKind::Idle {
            self.state = PetState::Sleep;
            self.state_timer = 6.0;
            self.vx = 0.0;
            self.say("Zzz... 12 hours max...", 2.5, false);
            return;
        }

        if ctx.activity == ActivityKind::Gaming {
            self.vx = 0.0;
            if rng.gen_bool(0.14) {
                self.state = PetState::CodeCheer;
                self.state_timer = rng.gen_range(2.2..3.4);
                self.say("Ooh! Look at that Crit! Nice!", 2.2, false);
            } else if rng.gen_bool(0.12) {
                self.state = PetState::Snack;
                self.state_timer = rng.gen_range(2.2..3.5);
                self.say("*slurp* Instant yakisoba buff!", 2.0, false);
            } else {
                self.state = PetState::BackseatGamer;
                self.state_timer = rng.gen_range(5.5..9.5);
                if rng.gen_bool(0.28) {
                    let callouts = [
                        "Position Hack! Flanking left!",
                        "Cooldowns! Watch your meter!",
                        "Don't greed! Heal UP!!",
                        "All-Out Attack chance! Go go go!",
                        "That hitbox was rigged!!",
                        "Lock in! I got your buffs~",
                    ];
                    let line = callouts[rng.gen_range(0..callouts.len())];
                    self.say(line, 2.3, false);
                }
            }
            return;
        }

        if ctx.music_playing && rng.gen_bool(0.55) {
            self.state = PetState::Vibe;
            self.state_timer = rng.gen_range(4.0..7.5);
            self.vx = 0.0;
            return;
        }

        if (ctx.video_playing || ctx.activity == ActivityKind::MediaVideo) && rng.gen_bool(0.55) {
            self.state = if rng.gen_bool(0.35) {
                PetState::Snack
            } else {
                PetState::Sit
            };
            self.state_timer = rng.gen_range(3.0..6.0);
            self.vx = 0.0;
            return;
        }

        if ctx.activity == ActivityKind::Coding && rng.gen_bool(0.72) {
            if rng.gen_bool(0.2) {
                self.state = PetState::CodeCheer;
                self.state_timer = rng.gen_range(2.5..4.0);
            } else {
                self.state = PetState::ChairGremlin;
                self.state_timer = rng.gen_range(5.5..9.5);
                if rng.gen_bool(0.22) {
                    self.say("Code's compiling... don't look at me!", 2.4, false);
                }
            }
            self.vx = 0.0;
            return;
        }

        if dist_cursor < 140.0 && rng.gen_bool(0.4) {
            self.state = PetState::PeekCursor;
            self.facing_left = (ctx.cursor_x as f32) < self.x;
            self.state_timer = rng.gen_range(1.8..3.2);
            self.vx = 0.0;
            return;
        }

        if allow_walk && rng.gen_bool(0.45) {
            self.state = PetState::Walk;
            let dir = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
            self.facing_left = dir < 0.0;
            self.vx = dir * rng.gen_range(75.0..120.0);
            self.state_timer = rng.gen_range(2.2..4.4);
        } else if rng.gen_bool(0.5) {
            self.state = PetState::Sit;
            self.vx = 0.0;
            self.state_timer = rng.gen_range(3.5..6.5);
        } else {
            self.state = PetState::ChairGremlin;
            self.vx = 0.0;
            self.state_timer = rng.gen_range(4.5..8.0);
        }
    }

    fn update_auto(&mut self, ctx: &ScreenContext, dt: f32, active_ledge: Option<&WindowLedge>) {
        if self.state == PetState::Walk && self.vy == 0.0 {
            self.x += self.vx * dt;

            // Keep inside perched window bounds and turn around gracefully
            if let Some(ledge) = active_ledge {
                let left_edge = (ledge.x + 34) as f32;
                let right_edge = (ledge.x + ledge.width - 34) as f32;
                if self.x <= left_edge {
                    self.x = left_edge;
                    if rand::thread_rng().gen_bool(0.25) {
                        // Hop off edge!
                        self.perched_window = None;
                        self.state = PetState::Fall;
                        self.vx = -65.0;
                    } else {
                        self.vx = self.vx.abs();
                        self.facing_left = false;
                    }
                } else if self.x >= right_edge {
                    self.x = right_edge;
                    if rand::thread_rng().gen_bool(0.25) {
                        self.perched_window = None;
                        self.state = PetState::Fall;
                        self.vx = 65.0;
                    } else {
                        self.vx = -self.vx.abs();
                        self.facing_left = true;
                    }
                }
            } else if self.jump_cooldown <= 0.0 {
                // Occasionally jump up onto a nearby window ledge above!
                if let Some(target) = find_jump_target_ledge(self.x, self.y, ctx) {
                    if rand::thread_rng().gen_bool(0.015) {
                        self.vy = -490.0;
                        self.vx = ((target.x + target.width / 2) as f32 - self.x).clamp(-130.0, 130.0);
                        self.facing_left = self.vx < 0.0;
                        self.jump_cooldown = 6.0;
                        self.say("Parkour!", 1.3, false);
                    }
                }
            }
        }

        self.update_ambient_react(ctx, dt, true);
    }

    pub fn current_frame(&self) -> AnimFrame {
        if self.state == PetState::Dragged {
            return AnimFrame::Dragged;
        }
        if self.vy < -30.0 && self.state != PetState::Fall {
            return AnimFrame::Walk1;
        }

        let t = self.anim_timer;
        if self.vy > 80.0 && self.state == PetState::Fall {
            return if ((t * 9.0) as usize).is_multiple_of(2) {
                AnimFrame::Fall0
            } else {
                AnimFrame::Fall1
            };
        }
        match self.state {
            PetState::Walk => {
                let fps = (self.vx.abs() / 18.0).clamp(4.5, 6.5);
                match ((t * fps) as usize) % 4 {
                    0 => AnimFrame::Walk0,
                    1 => AnimFrame::Walk1,
                    2 => AnimFrame::Walk2,
                    _ => AnimFrame::Walk3,
                }
            }
            PetState::Sit => {
                if ((t * 2.2) as usize).is_multiple_of(2) {
                    AnimFrame::Sit0
                } else {
                    AnimFrame::Sit1
                }
            }
            PetState::Vibe => {
                if ((t * 3.8) as usize).is_multiple_of(2) {
                    AnimFrame::Vibe0
                } else {
                    AnimFrame::Vibe1
                }
            }
            PetState::CodeCheer => {
                if ((t * 2.0) as usize) % 4 == 3 {
                    AnimFrame::Code1
                } else {
                    AnimFrame::Code0
                }
            }
            PetState::BackseatGamer => match ((t * 7.0) as usize) % 4 {
                0 => AnimFrame::Gamer0,
                1 => AnimFrame::Gamer1,
                2 => AnimFrame::Gamer2,
                _ => AnimFrame::Gamer3,
            },
            PetState::ChairGremlin => match ((t * 8.0) as usize) % 8 {
                0 => AnimFrame::Chair0,
                1 => AnimFrame::Chair1,
                2 => AnimFrame::Chair2,
                3 => AnimFrame::Chair3,
                4 => AnimFrame::Chair4,
                5 => AnimFrame::Chair5,
                6 => AnimFrame::Chair6,
                _ => AnimFrame::Chair7,
            },
            PetState::Snack => match ((t * 5.0) as usize) % 4 {
                0 => AnimFrame::Snack0,
                1 => AnimFrame::Snack1,
                2 => AnimFrame::Snack2,
                _ => AnimFrame::Snack1,
            },
            PetState::Caught => match self.caught_phase {
                CaughtPhase::Freeze => AnimFrame::Caught0,
                CaughtPhase::Hide => {
                    if ((t * 6.0) as usize).is_multiple_of(2) {
                        AnimFrame::Caught1
                    } else {
                        AnimFrame::Caught2
                    }
                }
                CaughtPhase::SlowPeek => {
                    if self.state_timer > 0.45 {
                        AnimFrame::Caught3
                    } else {
                        AnimFrame::Caught0
                    }
                }
            },
            PetState::Sleep => {
                if ((t * 1.2) as usize).is_multiple_of(2) {
                    AnimFrame::Sleep0
                } else {
                    AnimFrame::Sleep1
                }
            }
            PetState::Happy => AnimFrame::Happy,
            PetState::PeekCursor => AnimFrame::Peek,
            PetState::Dragged => AnimFrame::Dragged,
            PetState::Fall => {
                if ((t * 9.0) as usize).is_multiple_of(2) {
                    AnimFrame::Fall0
                } else {
                    AnimFrame::Fall1
                }
            }
            PetState::Land => {
                if self.state_timer > 0.18 {
                    AnimFrame::Land0
                } else {
                    AnimFrame::Land1
                }
            }
        }
    }

    pub fn render(&self, canvas: &mut Canvas, sheet: &SpriteSheet, local_x: i32, local_y: i32) {
        canvas.clear();

        // Ground shadow when resting on floor or window ledge
        if self.state != PetState::Dragged && self.vy.abs() < 40.0 {
            canvas.draw_shadow_oval(local_x, local_y - 2, 24, 5, 110);
        }

        let frame = self.current_frame();
        canvas.draw_sprite(sheet, frame, local_x, local_y, PIXEL_SCALE, self.facing_left);

        // Floating emotes/particles
        for p in &self.particles {
            canvas.draw_particle(p, local_x, local_y);
        }

        // Pixel speech bubble above her head with progressive typewriter reveal
        if let Some(msg) = &self.speech_text {
            let fade = if self.speech_timer < 0.4 {
                ((self.speech_timer / 0.4).clamp(0.0, 1.0) * 255.0) as u8
            } else {
                255
            };
            let elapsed = (self.speech_max - self.speech_timer).max(0.0);
            let total_chars = msg.chars().count();
            let visible = ((elapsed * 34.0).floor() as usize + 1).min(total_chars);
            let head_top = local_y - (sheet.frame_h as i32 - 4) * PIXEL_SCALE;
            canvas.draw_speech_bubble(msg, Some(visible), local_x, head_top, fade);
        }
    }
}

fn find_landing_ledge(x: f32, prev_y: f32, new_y: f32, ctx: &ScreenContext) -> Option<WindowLedge> {
    let mut best: Option<WindowLedge> = None;
    for ledge in &ctx.ledges {
        if ledge.y < ctx.reserved_top + 115 {
            continue;
        }
        let ly = ledge.y as f32;
        if prev_y <= ly + 6.0
            && new_y >= ly - 2.0
            && x >= (ledge.x + 24) as f32
            && x <= (ledge.x + ledge.width - 24) as f32
            && best.as_ref().is_none_or(|b| ledge.y < b.y)
        {
            best = Some(ledge.clone());
        }
    }
    best
}

fn find_jump_target_ledge(x: f32, y: f32, ctx: &ScreenContext) -> Option<WindowLedge> {
    ctx.ledges
        .iter()
        .filter(|l| {
            l.y >= ctx.reserved_top + 115
                && (y - l.y as f32) > 40.0
                && (y - l.y as f32) < 230.0
                && x >= (l.x - 90) as f32
                && x <= (l.x + l.width + 90) as f32
        })
        .min_by_key(|l| (y as i32 - l.y).abs())
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sprite::{CANVAS_H, CANVAS_W};

    #[test]
    fn sprite_sheet_loads_and_renders_non_empty_pixels() {
        let sheet = SpriteSheet::load(None).expect("embedded sprite sheet");
        assert_eq!(sheet.frame_w, 68);
        assert_eq!(sheet.frame_h, 68);
        let mut buf = vec![0u8; (CANVAS_W * CANVAS_H * 4) as usize];
        let mut canvas = Canvas {
            data: &mut buf,
            width: CANVAS_W as i32,
            height: CANVAS_H as i32,
        };
        let pet = Pet::new(1536, 960);
        pet.render(&mut canvas, &sheet, 130, 200);
        assert!(buf.iter().any(|&b| b != 0));
    }

    #[test]
    fn petting_and_mode_cycle_update_state() {
        let mut pet = Pet::new(1536, 960);
        pet.on_petted();
        assert_eq!(pet.state, PetState::Happy);
        assert!(!pet.particles.is_empty());

        assert_eq!(pet.mode, PetMode::Auto);
        pet.on_right_click();
        assert_eq!(pet.mode, PetMode::FollowCursor);
        pet.on_right_click();
        assert_eq!(pet.mode, PetMode::Perch);
        pet.on_right_click();
        assert_eq!(pet.mode, PetMode::Auto);
    }

    #[test]
    fn lands_on_window_ledge_when_falling() {
        let mut ctx = ScreenContext::default();
        ctx.ledges.push(WindowLedge {
            address: "0x1".into(),
            x: 100,
            y: 300,
            width: 500,
            class: "kitty".into(),
        });
        let found = find_landing_ledge(250.0, 295.0, 305.0, &ctx);
        assert!(found.is_some());
        assert_eq!(found.unwrap().address, "0x1");
    }

    #[test]
    fn falling_and_landing_do_not_switch_mid_air() {
        let mut pet = Pet::new(1536, 960);
        let mut ctx = ScreenContext {
            screen_w: 1536,
            screen_h: 960,
            cursor_x: 100,
            cursor_y: 800,
            last_event_seq: 1,
            last_event_kind: Some(ScreenEvent::MusicStarted("Last Surprise".into())),
            ..ScreenContext::default()
        };

        // Drop high above floor in FollowCursor mode with expired timer & incoming screen events
        pet.mode = PetMode::FollowCursor;
        pet.y = 300.0;
        pet.state_timer = 0.0;
        pet.end_drag(&ctx, 0.0, 0.0);
        assert_eq!(pet.state, PetState::Fall);

        pet.update(&ctx);
        assert_eq!(pet.state, PetState::Fall);
        assert!(matches!(pet.current_frame(), AnimFrame::Fall0 | AnimFrame::Fall1));

        // Switch mode to Auto with expired timer and Coding window change while still falling
        pet.mode = PetMode::Auto;
        pet.state_timer = 0.0;
        ctx.last_event_seq = 2;
        ctx.last_event_kind = Some(ScreenEvent::WindowChanged {
            class: "code".into(),
            title: "main.rs".into(),
        });
        pet.update(&ctx);
        assert_eq!(pet.state, PetState::Fall);
        assert!(matches!(pet.current_frame(), AnimFrame::Fall0 | AnimFrame::Fall1));

        // Simulate reaching floor impact in FollowCursor mode -> enters Land and renders Land0/Land1
        pet.mode = PetMode::FollowCursor;
        pet.y = 959.5;
        pet.vy = 1000.0;
        pet.update(&ctx);
        assert_eq!(pet.state, PetState::Land);
        assert!(matches!(pet.current_frame(), AnimFrame::Land0 | AnimFrame::Land1));
    }

    #[test]
    fn gaming_window_activates_backseat_copilot() {
        let sheet = SpriteSheet::load(None).expect("embedded sprite sheet");
        let mut pet = Pet::new(1536, 960);
        let ctx = ScreenContext {
            screen_w: 1536,
            screen_h: 960,
            activity: ActivityKind::Gaming,
            last_event_seq: 1,
            last_event_kind: Some(ScreenEvent::WindowChanged {
                class: "steam_app_1245620".into(),
                title: "ELDEN RING".into(),
            }),
            ..ScreenContext::default()
        };

        pet.update(&ctx);
        assert_eq!(pet.state, PetState::BackseatGamer);
        assert!(matches!(
            pet.current_frame(),
            AnimFrame::Gamer0 | AnimFrame::Gamer1 | AnimFrame::Gamer2 | AnimFrame::Gamer3
        ));
        for frame in [
            AnimFrame::Gamer0,
            AnimFrame::Gamer1,
            AnimFrame::Gamer2,
            AnimFrame::Gamer3,
        ] {
            assert!(sheet.frame_pixels(frame).iter().any(|p| p[3] > 0));
        }
    }

    #[test]
    fn throwing_pet_carries_velocity_and_bounces_off_walls() {
        let mut pet = Pet::new(1536, 960);
        let ctx = ScreenContext {
            screen_w: 1536,
            screen_h: 960,
            reserved_top: 34,
            ..ScreenContext::default()
        };

        // Throw strongly toward top-left corner
        pet.x = 45.0;
        pet.y = 125.0;
        pet.end_drag(&ctx, -1200.0, -900.0);
        assert_eq!(pet.state, PetState::Fall);
        assert!(pet.vx < -500.0);
        assert!(pet.vy < -500.0);
        assert!(matches!(pet.current_frame(), AnimFrame::Fall0 | AnimFrame::Fall1));

        // Update step hits left wall & ceiling and bounces back inward/downward
        std::thread::sleep(std::time::Duration::from_millis(5));
        pet.update(&ctx);
        assert_eq!(pet.x, 44.0);
        assert!(pet.vx > 0.0);
        assert_eq!(pet.y, (ctx.reserved_top + 90) as f32);
        assert!(pet.vy > 0.0);
    }
}
