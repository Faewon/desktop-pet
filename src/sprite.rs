use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use serde::Deserialize;

pub const CANVAS_W: u32 = 340;
pub const CANVAS_H: u32 = 248;
pub const PIXEL_SCALE: i32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimFrame {
    Walk0,
    Walk1,
    Walk2,
    Walk3,
    Sit0,
    Sit1,
    Vibe0,
    Vibe1,
    Code0,
    Code1,
    Gamer0,
    Gamer1,
    Gamer2,
    Gamer3,
    Chair0,
    Chair1,
    Chair2,
    Chair3,
    Chair4,
    Chair5,
    Chair6,
    Chair7,
    Snack0,
    Snack1,
    Snack2,
    Caught0,
    Caught1,
    Caught2,
    Caught3,
    Fall0,
    Fall1,
    Land0,
    Land1,
    Sleep0,
    Sleep1,
    Dragged,
    Happy,
    Peek,
}

impl AnimFrame {
    pub fn key(self) -> &'static str {
        match self {
            AnimFrame::Walk0 => "walk_0",
            AnimFrame::Walk1 => "walk_1",
            AnimFrame::Walk2 => "walk_2",
            AnimFrame::Walk3 => "walk_3",
            AnimFrame::Sit0 => "sit_0",
            AnimFrame::Sit1 => "sit_1",
            AnimFrame::Vibe0 => "vibe_0",
            AnimFrame::Vibe1 => "vibe_1",
            AnimFrame::Code0 => "code_0",
            AnimFrame::Code1 => "code_1",
            AnimFrame::Gamer0 => "gamer_0",
            AnimFrame::Gamer1 => "gamer_1",
            AnimFrame::Gamer2 => "gamer_2",
            AnimFrame::Gamer3 => "gamer_3",
            AnimFrame::Chair0 => "chair_0",
            AnimFrame::Chair1 => "chair_1",
            AnimFrame::Chair2 => "chair_2",
            AnimFrame::Chair3 => "chair_3",
            AnimFrame::Chair4 => "chair_4",
            AnimFrame::Chair5 => "chair_5",
            AnimFrame::Chair6 => "chair_6",
            AnimFrame::Chair7 => "chair_7",
            AnimFrame::Snack0 => "snack_0",
            AnimFrame::Snack1 => "snack_1",
            AnimFrame::Snack2 => "snack_2",
            AnimFrame::Caught0 => "caught_0",
            AnimFrame::Caught1 => "caught_1",
            AnimFrame::Caught2 => "caught_2",
            AnimFrame::Caught3 => "caught_3",
            AnimFrame::Fall0 => "fall_0",
            AnimFrame::Fall1 => "fall_1",
            AnimFrame::Land0 => "land_0",
            AnimFrame::Land1 => "land_1",
            AnimFrame::Sleep0 => "sleep_0",
            AnimFrame::Sleep1 => "sleep_1",
            AnimFrame::Dragged => "dragged",
            AnimFrame::Happy => "happy",
            AnimFrame::Peek => "peek",
        }
    }
}

#[derive(Debug, Deserialize)]
struct SpriteMeta {
    frame_width: u32,
    frame_height: u32,
    frames: HashMap<String, usize>,
}

pub struct SpriteSheet {
    pub frame_w: u32,
    pub frame_h: u32,
    frames: Vec<Vec<[u8; 4]>>,
    indices: HashMap<String, usize>,
}

const DEFAULT_PNG: &[u8] = include_bytes!("../assets/futaba.png");
const DEFAULT_META: &str = include_str!("../assets/futaba.json");

impl SpriteSheet {
    pub fn load(custom_png: Option<&Path>) -> Result<Self, String> {
        let meta_str = if let Some(path) = custom_png {
            let sidecar = path.with_extension("json");
            if sidecar.exists() {
                Some(std::fs::read_to_string(&sidecar).map_err(|e| format!("Read {}: {e}", sidecar.display()))?)
            } else {
                None
            }
        } else {
            None
        };
        let meta: SpriteMeta = serde_json::from_str(meta_str.as_deref().unwrap_or(DEFAULT_META))
            .map_err(|e| format!("Failed to parse sprite json: {e}"))?;

        let (width, height, rgba) = if let Some(path) = custom_png {
            let file = File::open(path).map_err(|e| format!("Open {}: {e}", path.display()))?;
            decode_png(BufReader::new(file))?
        } else {
            decode_png(DEFAULT_PNG)?
        };

        let fw = meta.frame_width;
        let fh = meta.frame_height;
        if fw == 0 || fh == 0 || width < fw || height < fh {
            return Err("Invalid sprite sheet dimensions".into());
        }

        let cols = (width / fw) as usize;
        let rows = (height / fh) as usize;
        let mut frames = Vec::with_capacity(cols * rows);

        for r in 0..rows {
            for c in 0..cols {
                let mut pixels = vec![[0u8; 4]; (fw * fh) as usize];
                for y in 0..fh {
                    for x in 0..fw {
                        let sx = (c as u32 * fw + x) as usize;
                        let sy = (r as u32 * fh + y) as usize;
                        let idx = (sy * width as usize + sx) * 4;
                        pixels[(y * fw + x) as usize] = [
                            rgba[idx],
                            rgba[idx + 1],
                            rgba[idx + 2],
                            rgba[idx + 3],
                        ];
                    }
                }
                frames.push(pixels);
            }
        }

        Ok(Self {
            frame_w: fw,
            frame_h: fh,
            frames,
            indices: meta.frames,
        })
    }

    pub fn frame_pixels(&self, anim: AnimFrame) -> &[[u8; 4]] {
        let fallback_key = match anim {
            AnimFrame::Chair0
            | AnimFrame::Chair2
            | AnimFrame::Chair4
            | AnimFrame::Chair6 => "code_0",
            AnimFrame::Chair1
            | AnimFrame::Chair3
            | AnimFrame::Chair5
            | AnimFrame::Chair7 => "code_1",
            AnimFrame::Snack0 | AnimFrame::Snack1 | AnimFrame::Snack2 => "happy",
            AnimFrame::Caught0 | AnimFrame::Caught1 | AnimFrame::Caught2 | AnimFrame::Caught3 => {
                "peek"
            }
            AnimFrame::Fall0 | AnimFrame::Fall1 => "dragged",
            AnimFrame::Land0 => "walk_1",
            AnimFrame::Land1 => "sit_0",
            AnimFrame::Gamer0 | AnimFrame::Gamer2 => "code_0",
            AnimFrame::Gamer1 | AnimFrame::Gamer3 => "code_1",
            _ => anim.key(),
        };
        let idx = self
            .indices
            .get(anim.key())
            .or_else(|| self.indices.get(fallback_key))
            .copied()
            .unwrap_or(0)
            .min(self.frames.len().saturating_sub(1));
        &self.frames[idx]
    }
}

fn decode_png<R: std::io::Read>(reader: R) -> Result<(u32, u32, Vec<u8>), String> {
    let mut decoder = png::Decoder::new(reader);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| format!("PNG info error: {e}"))?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| format!("PNG frame error: {e}"))?;
    let bytes = &buf[..info.buffer_size()];

    let rgba = match info.color_type {
        png::ColorType::Rgba => bytes.to_vec(),
        png::ColorType::Rgb => {
            let mut out = Vec::with_capacity((info.width * info.height * 4) as usize);
            for chunk in bytes.as_chunks::<3>().0 {
                out.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
            out
        }
        png::ColorType::GrayscaleAlpha => {
            let mut out = Vec::with_capacity((info.width * info.height * 4) as usize);
            for chunk in bytes.as_chunks::<2>().0 {
                out.extend_from_slice(&[chunk[0], chunk[0], chunk[0], chunk[1]]);
            }
            out
        }
        png::ColorType::Grayscale => {
            let mut out = Vec::with_capacity((info.width * info.height * 4) as usize);
            for &g in bytes {
                out.extend_from_slice(&[g, g, g, 255]);
            }
            out
        }
        other => return Err(format!("Unsupported PNG color type: {other:?}")),
    };

    Ok((info.width, info.height, rgba))
}

#[derive(Debug, Clone, Copy)]
pub enum ParticleKind {
    Heart,
    MusicNote,
    Sparkle,
    Zzz,
    CodeBit,
    Alert,
}

#[derive(Debug, Clone)]
pub struct Particle {
    pub kind: ParticleKind,
    pub variant: u8,
    pub dx: f32,
    pub dy: f32,
    pub vx: f32,
    pub vy: f32,
    pub age: f32,
    pub max_age: f32,
}

/// ARGB8888 premultiplied pixel buffer helper.
pub struct Canvas<'a> {
    pub data: &'a mut [u8],
    pub width: i32,
    pub height: i32,
}

impl<'a> Canvas<'a> {
    pub fn clear(&mut self) {
        self.data.fill(0);
    }

    #[inline]
    pub fn blend_pixel(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8, a: u8) {
        if x < 0 || y < 0 || x >= self.width || y >= self.height || a == 0 {
            return;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        if a == 255 {
            // Wayland ARGB8888 little-endian = [B, G, R, A]
            self.data[idx] = b;
            self.data[idx + 1] = g;
            self.data[idx + 2] = r;
            self.data[idx + 3] = 255;
            return;
        }
        let sa = a as u32;
        let inv = 255 - sa;
        let sr = (r as u32 * sa) / 255;
        let sg = (g as u32 * sa) / 255;
        let sb = (b as u32 * sa) / 255;

        let db = self.data[idx] as u32;
        let dg = self.data[idx + 1] as u32;
        let dr = self.data[idx + 2] as u32;
        let da = self.data[idx + 3] as u32;

        self.data[idx] = (sb + (db * inv) / 255).min(255) as u8;
        self.data[idx + 1] = (sg + (dg * inv) / 255).min(255) as u8;
        self.data[idx + 2] = (sr + (dr * inv) / 255).min(255) as u8;
        self.data[idx + 3] = (sa + (da * inv) / 255).min(255) as u8;
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, rgba: [u8; 4]) {
        let [r, g, b, a] = rgba;
        for py in y..(y + h) {
            for px in x..(x + w) {
                self.blend_pixel(px, py, r, g, b, a);
            }
        }
    }

    pub fn draw_shadow_oval(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, alpha: u8) {
        for dy in -ry..=ry {
            for dx in -rx..=rx {
                let nx = dx as f32 / rx.max(1) as f32;
                let ny = dy as f32 / ry.max(1) as f32;
                let dist = nx * nx + ny * ny;
                if dist <= 1.0 {
                    let falloff = ((1.0 - dist) * alpha as f32) as u8;
                    self.blend_pixel(cx + dx, cy + dy, 24, 14, 34, falloff);
                }
            }
        }
    }

    pub fn draw_sprite(
        &mut self,
        sheet: &SpriteSheet,
        anim: AnimFrame,
        foot_x: i32,
        foot_y: i32,
        scale: i32,
        flip_x: bool,
    ) {
        let pixels = sheet.frame_pixels(anim);
        let fw = sheet.frame_w as i32;
        let fh = sheet.frame_h as i32;
        let draw_w = fw * scale;
        let origin_x = foot_x - draw_w / 2;
        let origin_y = foot_y - (fh - 2) * scale;

        for sy in 0..fh {
            for sx in 0..fw {
                let src_x = if flip_x { fw - 1 - sx } else { sx };
                let [r, g, b, a] = pixels[(sy * fw + src_x) as usize];
                if a == 0 {
                    continue;
                }
                let px0 = origin_x + sx * scale;
                let py0 = origin_y + sy * scale;
                self.fill_rect(px0, py0, scale, scale, [r, g, b, a]);
            }
        }
    }

    pub fn draw_particle(&mut self, p: &Particle, anchor_x: i32, anchor_y: i32) {
        let t = (p.age / p.max_age).clamp(0.0, 1.0);
        let alpha = ((1.0 - t * t) * 255.0) as u8;
        let x = anchor_x + p.dx as i32;
        let y = anchor_y + p.dy as i32;

        if matches!(p.kind, ParticleKind::CodeBit) {
            match p.variant % 4 {
                0 => {
                    // 3x3 neon-green UI square
                    self.fill_rect(x, y, 3, 3, [68, 255, 68, alpha]);
                    self.fill_rect(x + 1, y + 1, 1, 1, [200, 255, 160, alpha]);
                }
                1 => {
                    // Binary '1' glyph (3x5)
                    for (ry, row) in [".#.", "##.", ".#.", ".#.", "###"].iter().enumerate() {
                        for (rx, ch) in row.bytes().enumerate() {
                            if ch == b'#' {
                                self.fill_rect(x + rx as i32, y + ry as i32, 1, 1, [136, 255, 40, alpha]);
                            }
                        }
                    }
                }
                2 => {
                    // Binary '0' glyph (3x5)
                    for (ry, row) in ["###", "#.#", "#.#", "#.#", "###"].iter().enumerate() {
                        for (rx, ch) in row.bytes().enumerate() {
                            if ch == b'#' {
                                self.fill_rect(x + rx as i32, y + ry as i32, 1, 1, [68, 255, 68, alpha]);
                            }
                        }
                    }
                }
                _ => {
                    // Prompt chevron '>' (3x3)
                    for (ry, row) in ["#..", ".#.", "#.."].iter().enumerate() {
                        for (rx, ch) in row.bytes().enumerate() {
                            if ch == b'#' {
                                self.fill_rect(x + rx as i32, y + ry as i32, 1, 1, [160, 255, 100, alpha]);
                            }
                        }
                    }
                }
            }
            return;
        }

        let s = 2;
        let (pattern, color): (&[&str], (u8, u8, u8)) = match p.kind {
            ParticleKind::Heart => (
                &[".##.##.", "#######", "#######", ".#####.", "..###..", "...#..."],
                (255, 102, 146),
            ),
            ParticleKind::MusicNote => (
                &["...###.", "...#.#.", "...#...", ".###...", "###....", ".#....."],
                (118, 228, 252),
            ),
            ParticleKind::Sparkle => (
                &["...#...", "...#...", ".#####.", "...#...", "...#..."],
                (255, 224, 110),
            ),
            ParticleKind::Zzz => (
                &["####.", "..#..", ".#...", "####."],
                (190, 204, 255),
            ),
            ParticleKind::Alert => (
                &[".##.", ".##.", ".##.", "....", ".##."],
                (255, 92, 92),
            ),
            ParticleKind::CodeBit => unreachable!(),
        };

        for (ry, row) in pattern.iter().enumerate() {
            for (rx, ch) in row.bytes().enumerate() {
                if ch == b'#' {
                    self.fill_rect(
                        x + rx as i32 * s,
                        y + ry as i32 * s,
                        s,
                        s,
                        [color.0, color.1, color.2, alpha],
                    );
                }
            }
        }
    }

    pub fn draw_speech_bubble(
        &mut self,
        text: &str,
        visible_chars: Option<usize>,
        anchor_x: i32,
        top_y: i32,
        alpha: u8,
    ) {
        if text.is_empty() || alpha == 0 {
            return;
        }
        let max_cols = 22usize;
        let max_lines = 4usize;
        let lines = wrap_text(text, max_cols, max_lines);
        if lines.is_empty() {
            return;
        }
        let total_wrapped_chars: usize = lines.iter().map(|l| l.chars().count()).sum();
        let shown_limit = visible_chars.unwrap_or(total_wrapped_chars).min(total_wrapped_chars);

        let scale = 2;
        let char_w = 6; // 5px glyph + 1px gap
        let line_h = 9 * scale; // 7px glyph * scale + 4px line gap
        let max_line_chars = lines
            .iter()
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(1)
            .max(2) as i32;
        let text_w = (max_line_chars * char_w - 1).max(11) * scale + 4;
        let text_h = (lines.len() as i32 - 1) * line_h + 7 * scale;
        let pad_x = 10;
        let pad_y = 8;
        let box_w = (text_w + pad_x * 2).min(self.width - 8).max(40);
        let box_h = text_h + pad_y * 2;

        let max_bx = (self.width - box_w - 4).max(4);
        let bx = (anchor_x - box_w / 2).clamp(4, max_bx);
        let by = (top_y - box_h - 10).max(4);

        let border = [62u8, 42, 78, alpha];
        let fill = [255u8, 248, 252, ((alpha as u16 * 244) / 255) as u8];
        let shadow = [36u8, 22, 48, ((alpha as u16 * 90) / 255) as u8];

        // Drop shadow
        self.fill_rect(bx + 2, by + 2, box_w, box_h, shadow);

        // Fill body
        self.fill_rect(bx + 2, by + 2, box_w - 4, box_h - 4, fill);

        // Pixel-rounded border
        self.fill_rect(bx + 2, by, box_w - 4, 2, border);
        self.fill_rect(bx + 2, by + box_h - 2, box_w - 4, 2, border);
        self.fill_rect(bx, by + 2, 2, box_h - 4, border);
        self.fill_rect(bx + box_w - 2, by + 2, 2, box_h - 4, border);

        // Little pointer tail toward pet
        let tx_max = (bx + box_w - 10).max(bx + 8);
        let tx = anchor_x.clamp(bx + 8, tx_max);
        self.fill_rect(tx - 2, by + box_h - 2, 6, 4, fill);
        self.fill_rect(tx - 4, by + box_h, 2, 2, border);
        self.fill_rect(tx + 4, by + box_h, 2, 2, border);
        self.fill_rect(tx - 2, by + box_h + 2, 6, 2, border);
        self.fill_rect(tx - 2, by + box_h + 4, 4, 2, border);

        // Wrapped text with progressive typewriter reveal
        let mut remaining = shown_limit;
        let mut cursor_pos: Option<(i32, i32)> = None;
        for (line_idx, line) in lines.iter().enumerate() {
            let cy = by + pad_y + line_idx as i32 * line_h;
            let mut cx = bx + pad_x;
            for ch in line.chars() {
                if remaining == 0 {
                    if shown_limit < total_wrapped_chars && cursor_pos.is_none() {
                        cursor_pos = Some((cx, cy));
                    }
                    break;
                }
                remaining -= 1;
                let glyph = glyph_5x7(ch);
                for (row, bits) in glyph.iter().enumerate() {
                    for col in 0..5 {
                        if (bits >> (4 - col)) & 1 == 1 {
                            self.fill_rect(
                                cx + col * scale,
                                cy + row as i32 * scale,
                                scale,
                                scale,
                                [56, 36, 68, alpha],
                            );
                        }
                    }
                }
                cx += char_w * scale;
            }
            if remaining == 0 && shown_limit < total_wrapped_chars && cursor_pos.is_none() {
                cursor_pos = Some((cx, cy));
            }
            if remaining == 0 {
                break;
            }
        }

        // Subtle pixel cursor while text is generating
        if let Some((cur_x, cur_y)) = cursor_pos {
            if cur_x + 4 <= bx + box_w - 4 {
                self.fill_rect(cur_x, cur_y + 2, 3, 7 * scale - 2, [96, 178, 86, alpha]);
            }
        }
    }
}

pub fn wrap_text(text: &str, max_cols: usize, max_lines: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let max_cols = max_cols.max(4);
    let max_lines = max_lines.max(1);

    for raw_para in text.split('\n') {
        let mut current = String::new();
        for word in raw_para.split_whitespace() {
            let word_len = word.chars().count();
            if word_len > max_cols {
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                    if lines.len() >= max_lines {
                        return lines;
                    }
                }
                for chunk in word.chars().collect::<Vec<_>>().chunks(max_cols) {
                    lines.push(chunk.iter().collect());
                    if lines.len() >= max_lines {
                        return lines;
                    }
                }
                continue;
            }

            let needed = if current.is_empty() {
                word_len
            } else {
                current.chars().count() + 1 + word_len
            };
            if needed <= max_cols {
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(word);
            } else {
                lines.push(std::mem::take(&mut current));
                if lines.len() >= max_lines {
                    return lines;
                }
                current.push_str(word);
            }
        }
        if !current.is_empty() || lines.is_empty() {
            lines.push(current);
            if lines.len() >= max_lines {
                return lines;
            }
        }
    }
    lines.retain(|l| !l.is_empty());
    lines
}

fn glyph_5x7(c: char) -> [u8; 7] {
    match c.to_ascii_uppercase() {
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b11110, 0b10001, 0b10001, 0b10001, 0b11110],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' => [0b11100, 0b10010, 0b10001, 0b10001, 0b10001, 0b10010, 0b11100],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '0' => [0b01110, 0b10011, 0b10101, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b01110, 0b10000, 0b11110, 0b10001, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110],
        '!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100],
        '?' => [0b01110, 0b10001, 0b00001, 0b00110, 0b00100, 0b00000, 0b00100],
        '.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100],
        ',' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00100, 0b00100, 0b01000],
        ':' => [0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000],
        '-' | '—' | '–' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        '+' => [0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000],
        '*' => [0b00100, 0b10101, 0b01110, 0b11111, 0b01110, 0b10101, 0b00100],
        '/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000],
        '=' => [0b00000, 0b00000, 0b11111, 0b00000, 0b11111, 0b00000, 0b00000],
        '_' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b11111],
        '<' => [0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010],
        '>' => [0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000],
        '~' => [0b00000, 0b00000, 0b01001, 0b10110, 0b00000, 0b00000, 0b00000],
        '\'' => [0b00100, 0b00100, 0b01000, 0b00000, 0b00000, 0b00000, 0b00000],
        '"' => [0b01010, 0b01010, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
        '(' => [0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010],
        ')' => [0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000],
        '♪' | '♫' => [0b00111, 0b00101, 0b00100, 0b00100, 0b01100, 0b11100, 0b01000],
        '▶' => [0b10000, 0b11000, 0b11100, 0b11110, 0b11100, 0b11000, 0b10000],
        '♥' => [0b00000, 0b01010, 0b11111, 0b11111, 0b01110, 0b00100, 0b00000],
        _ => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_speech_bubbles_never_panic() {
        let mut buf = vec![0u8; (CANVAS_W * CANVAS_H * 4) as usize];
        let mut canvas = Canvas {
            data: &mut buf,
            width: CANVAS_W as i32,
            height: CANVAS_H as i32,
        };
        canvas.draw_speech_bubble("Gah! Don't sneak up!", Some(5), 170, 90, 255);
        canvas.draw_speech_bubble("N-No! Too bright! Put me down!", None, 40, 80, 255);
        canvas.draw_speech_bubble(&"X".repeat(120), None, 200, 80, 255);

        let wrapped = wrap_text("N-No! Too bright! Put me down!", 22, 4);
        assert_eq!(wrapped.len(), 2);
        assert!(wrapped.iter().all(|l| l.chars().count() <= 22));
    }
}
