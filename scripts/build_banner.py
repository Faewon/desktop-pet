#!/usr/bin/env python3
"""Generate assets/banner.png for the desktop-pet GitHub README."""

import json
import os
from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.dirname(__file__))
SHEET_PNG = os.path.join(ROOT, "assets", "futaba.png")
SHEET_JSON = os.path.join(ROOT, "assets", "futaba.json")
OUT_PATH = os.path.join(ROOT, "assets", "banner.png")

ORBITRON = "/usr/share/fonts/sddm-astronaut-theme/Orbitron Black.ttf"
MONO_BOLD = "/usr/share/fonts/TTF/JetBrainsMonoNLNerdFont-Bold.ttf"
MONO_REG = "/usr/share/fonts/TTF/JetBrainsMonoNLNerdFont-Regular.ttf"


def get_sprite(sheet, meta, name, scale=3, flip_x=False):
    fw, fh = meta["frame_width"], meta["frame_height"]
    idx = meta["frames"][name]
    crop = sheet.crop((idx * fw, 0, (idx + 1) * fw, fh))
    if flip_x:
        crop = crop.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
    return crop.resize((fw * scale, fh * scale), Image.Resampling.NEAREST)


def paste_grounded(canvas, sprite, center_x, foot_y, scale=3):
    # Sprites in futaba.png are grounded at y=65 out of 68
    ox = center_x - sprite.width // 2
    oy = foot_y - 66 * scale
    canvas.paste(sprite, (ox, oy), sprite)


def main():
    W, H = 1200, 340
    im = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    draw = ImageDraw.Draw(im)

    # Vertical gradient backdrop inside rounded card
    bg = Image.new("RGBA", (W, H), (18, 16, 30, 255))
    bdraw = ImageDraw.Draw(bg)
    for y in range(H):
        t = y / (H - 1)
        r = int(18 + 14 * t)
        g = int(16 + 10 * t)
        b = int(32 + 20 * t)
        bdraw.line([(0, y), (W, y)], fill=(r, g, b, 255))

    # Subtle cyber grid
    for gx in range(0, W, 32):
        bdraw.line([(gx, 0), (gx, H)], fill=(44, 38, 68, 45), width=1)
    for gy in range(0, H, 32):
        bdraw.line([(0, gy), (W, gy)], fill=(44, 38, 68, 45), width=1)

    # Soft multi-ring ambient glows behind hero text & sprites
    glow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    gdraw = ImageDraw.Draw(glow)
    for step in range(4):
        pad = step * 26
        a = 12 - step * 2
        gdraw.ellipse((520 - pad, 40 - pad, 960 + pad, 340 + pad), fill=(68, 255, 68, a))
        gdraw.ellipse((40 - pad, 20 - pad, 460 + pad, 290 + pad), fill=(238, 112, 24, a))
        gdraw.ellipse((860 - pad, 10 - pad, 1180 + pad, 260 + pad), fill=(120, 90, 255, a))
    bg = Image.alpha_composite(bg, glow)

    # Rounded mask
    mask = Image.new("L", (W, H), 0)
    ImageDraw.Draw(mask).rounded_rectangle((2, 2, W - 3, H - 3), radius=20, fill=255)
    im.paste(bg, (0, 0), mask)
    draw = ImageDraw.Draw(im)

    # Card border & subtle accent stripes
    draw.rounded_rectangle((2, 2, W - 3, H - 3), radius=20, outline=(82, 66, 118, 255), width=2)
    draw.rectangle((48, 42, 54, 294), fill=(238, 112, 24, 255))

    # Fonts
    f_title = ImageFont.truetype(ORBITRON, 66)
    f_sub = ImageFont.truetype(MONO_BOLD, 19)
    f_pill = ImageFont.truetype(MONO_BOLD, 14)
    f_small = ImageFont.truetype(MONO_REG, 13)

    # Top pill tag
    draw.rounded_rectangle((74, 44, 358, 72), radius=8, fill=(24, 44, 34, 235), outline=(68, 255, 68, 210), width=2)
    draw.text((88, 49), "ORACLE // HYPRLAND DESKTOP PET", font=f_pill, fill=(136, 255, 80, 255))

    # Hero Title with crisp Persona-style orange offset shadow
    draw.text((78, 92), "FUTABA", font=f_title, fill=(238, 112, 24, 255))
    draw.text((74, 88), "FUTABA", font=f_title, fill=(250, 248, 255, 255))

    draw.text((76, 168), "Wayland Layer-Overlay Companion in Rust", font=f_sub, fill=(214, 210, 236, 255))

    # Compact feature chips
    chips = [
        ("Layer::Overlay", (68, 255, 68, 255)),
        ("Window Perching", (255, 164, 52, 255)),
        ("Flick Physics", (118, 228, 252, 255)),
    ]
    cx = 76
    for label, col in chips:
        tw = int(draw.textlength(label, font=f_small))
        draw.rounded_rectangle((cx, 206, cx + tw + 22, 232), radius=6, fill=(32, 28, 52, 230), outline=col, width=1)
        draw.text((cx + 11, 211), label, font=f_small, fill=(238, 236, 248, 255))
        cx += tw + 32

    # Pixel speech bubble on bottom-left pointing right toward Futaba
    bx0, by0, bx1, by1 = 76, 254, 434, 294
    draw.rounded_rectangle((bx0 + 3, by0 + 3, bx1 + 3, by1 + 3), radius=8, fill=(12, 10, 20, 160))
    draw.rounded_rectangle((bx0, by0, bx1, by1), radius=8, fill=(252, 246, 252, 248), outline=(62, 42, 78, 255), width=2)
    draw.text((bx0 + 16, by0 + 11), "Oracle online! All systems green!", font=f_pill, fill=(56, 36, 68, 255))

    # Floating Hyprland-style window card on right side for Futaba to perch on
    wx0, wy0, wx1, wy1 = 875, 170, 1146, 304
    draw.rounded_rectangle((wx0 + 4, wy0 + 4, wx1 + 4, wy1 + 4), radius=10, fill=(10, 8, 18, 170))
    draw.rounded_rectangle((wx0, wy0, wx1, wy1), radius=10, fill=(26, 24, 42, 245), outline=(136, 255, 0, 210), width=2)
    draw.line([(wx0 + 2, wy0 + 26), (wx1 - 2, wy0 + 26)], fill=(62, 56, 92, 255), width=1)
    draw.ellipse((wx0 + 12, wy0 + 9, wx0 + 20, wy0 + 17), fill=(255, 96, 96, 255))
    draw.ellipse((wx0 + 26, wy0 + 9, wx0 + 34, wy0 + 17), fill=(255, 196, 64, 255))
    draw.ellipse((wx0 + 40, wy0 + 9, wx0 + 48, wy0 + 17), fill=(68, 255, 68, 255))
    draw.text((wx0 + 60, wy0 + 6), "kitty :: desktop-pet", font=f_small, fill=(176, 172, 206, 255))
    draw.text((wx0 + 16, wy0 + 40), "$ cargo build --release", font=f_small, fill=(136, 255, 80, 255))
    draw.text((wx0 + 16, wy0 + 62), "> Layer level 3 (overlay)", font=f_small, fill=(196, 194, 224, 255))
    draw.text((wx0 + 16, wy0 + 84), "> 9 passed; 0 failed", font=f_small, fill=(118, 228, 252, 255))

    # Ground baseline
    draw.line([(470, 314), (1152, 314)], fill=(76, 68, 112, 180), width=2)

    # Load actual Futaba sprites & composite cleanly
    sheet = Image.open(SHEET_PNG).convert("RGBA")
    meta = json.load(open(SHEET_JSON))

    # Shadow ovals
    sh = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    sdraw = ImageDraw.Draw(sh)
    sdraw.ellipse((525, 306, 625, 322), fill=(8, 6, 16, 150))
    sdraw.ellipse((680, 306, 815, 322), fill=(8, 6, 16, 150))
    sdraw.ellipse((960, 164, 1068, 178), fill=(8, 6, 16, 150))
    im = Image.alpha_composite(im, sh)

    gamer = get_sprite(sheet, meta, "gamer_2", scale=3)
    chair = get_sprite(sheet, meta, "chair_2", scale=3)
    hero_land = get_sprite(sheet, meta, "land_0", scale=3, flip_x=True)

    paste_grounded(im, gamer, 575, 314, scale=3)
    paste_grounded(im, chair, 745, 314, scale=3)
    paste_grounded(im, hero_land, 1012, 170, scale=3)

    # Decorative floating pixel particles (green code bits & heart)
    pdraw = ImageDraw.Draw(im)
    for px, py, col in [
        (788, 170, (68, 255, 68, 220)),
        (806, 146, (136, 255, 0, 200)),
        (824, 184, (200, 255, 160, 180)),
        (618, 118, (255, 102, 146, 230)),
        (932, 46, (255, 224, 110, 220)),
    ]:
        pdraw.rectangle((px, py, px + 5, py + 5), fill=col)

    im.save(OUT_PATH)
    print(f"Saved banner {OUT_PATH} ({W}x{H})")


if __name__ == "__main__":
    main()
