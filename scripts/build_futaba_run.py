#!/usr/bin/env python3
"""Assemble Futaba's complete sprite sheet:
- Idle / Blink
- 8-frame contraposto Run Cycle (walk_0..walk_7)
- Sit / Music Vibe / Code Cheer / Sleep / Happy
- 8-frame Chair Gremlin hacking loop (chair_0..chair_7)
- 3-frame Yakisoba Slurp fidget (snack_0..snack_2)
- 2-frame Caught Peeking behind chair (caught_0..caught_1)
- Panicked Curled Ball dragged state (dragged)
"""

import json
import os
import sys
import numpy as np
from PIL import Image, ImageDraw

sys.path.insert(0, os.path.dirname(__file__))
import process_generated_sheet as p

BASE_SHEET = (
    "/home/faewon/.gemini/antigravity-cli/brain/"
    "4532fa2f-dc2e-4f0f-9409-ce14f8ea7fee/"
    "futaba_sakura_sprite_sheet_v2_1795661663907.jpg"
)
RUN_SHEET = (
    "/home/faewon/.gemini/antigravity-cli/brain/"
    "4532fa2f-dc2e-4f0f-9409-ce14f8ea7fee/"
    "futaba_run_cycle_sheet_v3_1795662048058.jpg"
)
GREMLIN_SHEET = (
    "/home/faewon/.gemini/antigravity-cli/brain/"
    "4532fa2f-dc2e-4f0f-9409-ce14f8ea7fee/"
    "futaba_sprite_sheet_final_1795662526728.jpg"
)
LAND_SHEET = (
    "/home/faewon/.gemini/antigravity-cli/brain/"
    "4532fa2f-dc2e-4f0f-9409-ce14f8ea7fee/"
    "futaba_superhero_landing_sprites_v3_1795666725314.jpg"
)
GAMER_SHEET = (
    "/home/faewon/.gemini/antigravity-cli/brain/"
    "4532fa2f-dc2e-4f0f-9409-ce14f8ea7fee/"
    "futaba_gamer_poses_1x2_1795667016159.jpg"
)
OUT_DIR = os.path.join(os.path.dirname(os.path.dirname(__file__)), "assets")
TARGET = 68

# Palette sampled from Futaba sprites
OUTLINE = (24, 20, 28, 255)
SKIN = (254, 218, 186, 255)
SKIN_SHADE = (218, 166, 140, 255)
PARKA = (98, 125, 80, 255)
PARKA_LIGHT = (122, 148, 98, 255)
PARKA_DARK = (64, 84, 52, 255)
BOOT_DARK = (28, 26, 36, 255)
BOOT_MID = (52, 50, 64, 255)
BOOT_HIGH = (80, 82, 98, 255)
SMEAR = (96, 98, 120, 130)

HAIR_ORANGE = (238, 112, 24, 255)
HAIR_LIGHT = (255, 164, 52, 255)
NEON_GREEN = (68, 255, 68, 255)
ACID_LIME = (136, 255, 0, 255)
WHITE_GLARE = (245, 255, 245, 255)
LAPTOP_DARK = (44, 48, 56, 255)


def extract_grid_cells(path, expected_rows, expected_cols):
    rgb = np.array(Image.open(path).convert("RGB"))
    r, g, b = rgb[:, :, 0].astype(int), rgb[:, :, 1].astype(int), rgb[:, :, 2].astype(int)
    fg = ~((g - np.maximum(r, b) > 50) & (g > 140))
    rows = p.find_spans(fg.sum(axis=1), min_gap=4)
    assert len(rows) == expected_rows, f"{path}: expected {expected_rows} rows, got {rows}"

    raw_cells, heights = [], []
    for ry0, ry1 in rows:
        cols = p.find_spans(fg[ry0:ry1].sum(axis=0), min_gap=16)
        assert len(cols) == expected_cols, f"{path}: expected {expected_cols} cols, got {cols}"
        for cx0, cx1 in cols:
            rgba = p.remove_green_bg(
                rgb[
                    max(0, ry0 - 2) : min(rgb.shape[0], ry1 + 2),
                    max(0, cx0 - 2) : min(rgb.shape[1], cx1 + 2),
                ]
            )
            ys, _ = np.where(rgba[:, :, 3] > 128)
            heights.append(int(ys.max() - ys.min() + 1))
            raw_cells.append(rgba)
    return raw_cells, heights


def extract_base_poses():
    raw_cells, heights = extract_grid_cells(BASE_SHEET, 3, 4)
    ref_h = float(heights[0])
    return [p.normalize_cell(c, ref_h) for c in raw_cells], raw_cells, ref_h


def extract_run_bases():
    rgb = np.array(Image.open(RUN_SHEET).convert("RGB"))
    r, g, b = rgb[:, :, 0].astype(int), rgb[:, :, 1].astype(int), rgb[:, :, 2].astype(int)
    fg = ~((g - np.maximum(r, b) > 100) & (g > 160) & (r < 85) & (b < 85))
    rows = p.find_spans(fg.sum(axis=1), min_gap=6)
    scale = 61.0 / (373 - 16 + 1)

    def place_cell(ry0, ry1, cx0, cx1, top_y):
        rgba = p.remove_green_bg(rgb[ry0:ry1, cx0:cx1])
        ys, xs = np.where(rgba[:, :, 3] > 128)
        crop = Image.fromarray(rgba[ys.min() : ys.max() + 1, xs.min() : xs.max() + 1])
        nw = int(round(crop.width * scale))
        nh = int(round(crop.height * scale))
        small = crop.resize((nw, nh), Image.Resampling.NEAREST)
        arr = np.array(small)
        face_xs = np.where(arr[10:25, :, 3] > 128)[1]
        ox = 50 - int(face_xs.max())
        canvas = Image.new("RGBA", (TARGET, TARGET), (0, 0, 0, 0))
        canvas.paste(small, (ox, top_y))
        return canvas

    ry0, ry1 = rows[0]
    cols0 = p.find_spans(fg[ry0:ry1].sum(axis=0), min_gap=16)
    ry2, ry3 = rows[1]
    cols1 = p.find_spans(fg[ry2:ry3].sum(axis=0), min_gap=16)

    return {
        "contact": place_cell(ry0, ry1, cols0[0][0], cols0[0][1], 5),
        "down_r": place_cell(ry0, ry1, cols0[1][0], cols0[1][1], 9),
        "down_l": place_cell(ry2, ry3, cols1[1][0], cols1[1][1], 8),
        "air": place_cell(ry0, ry1, cols0[3][0], cols0[3][1], 2),
    }


def put_px(arr, x, y, col):
    if 0 <= x < TARGET and 0 <= y < TARGET:
        arr[y, x] = col


def capsule_pts(x0, y0, x1, y1, radius):
    steps = max(abs(x1 - x0), abs(y1 - y0), 1) * 3
    pts = set()
    for i in range(steps + 1):
        t = i / steps
        cx = int(round(x0 + (x1 - x0) * t))
        cy = int(round(y0 + (y1 - y0) * t))
        for dy in range(-radius, radius + 1):
            for dx in range(-radius, radius + 1):
                if dx * dx + dy * dy <= radius * radius + 1:
                    pts.add((cx + dx, cy + dy))
    return pts


def draw_unified_leg(arr, hip_x, hip_y, knee_x, knee_y, ankle_x, ankle_y, toe_dx, toe_dy, far=False, smear_dx=0):
    skin_pts = capsule_pts(hip_x, hip_y, knee_x, knee_y, 2) | capsule_pts(knee_x, knee_y, ankle_x, ankle_y, 1)
    boot_pts = set()
    for dy in range(0, 6):
        for dx in range(-2, 3):
            boot_pts.add((ankle_x + dx, ankle_y + dy))
    tx = ankle_x + toe_dx
    ty = ankle_y + 5 + toe_dy
    steps = max(abs(toe_dx), abs(toe_dy), 1) * 2
    for i in range(steps + 1):
        t = i / steps
        cx = int(round(ankle_x + (tx - ankle_x) * t))
        cy = int(round(ankle_y + 4 + (ty - (ankle_y + 4)) * t))
        for dy in (-1, 0, 1):
            for dx in (-2, -1, 0, 1, 2):
                boot_pts.add((cx + dx, cy + dy))
    all_pts = skin_pts | boot_pts

    if smear_dx != 0:
        for sy in range(ankle_y + 2, ankle_y + 6):
            for sx in range(min(0, smear_dx), max(0, smear_dx) + 1):
                if (ankle_x + sx, sy) not in all_pts:
                    put_px(arr, ankle_x + sx, sy, SMEAR)

    for px, py in all_pts:
        for ody in (-1, 0, 1):
            for odx in (-1, 0, 1):
                if (px + odx, py + ody) not in all_pts and py + ody >= hip_y + 1:
                    put_px(arr, px + odx, py + ody, OUTLINE)

    base_skin = SKIN_SHADE if far else SKIN
    for px, py in skin_pts:
        if (px, py) not in boot_pts:
            put_px(arr, px, py, SKIN_SHADE if (far or px < min(hip_x, knee_x)) else base_skin)

    boot_fill = BOOT_DARK if far else BOOT_MID
    hi = BOOT_MID if far else BOOT_HIGH
    for px, py in boot_pts:
        put_px(arr, px, py, boot_fill)
    put_px(arr, ankle_x + 1, ankle_y + 1, hi)
    put_px(arr, ankle_x + 1, ankle_y + 3, hi)
    put_px(arr, tx, ty - 1, hi)


def draw_near_arm_forward(arr, sh_x, sh_y, elb_x, elb_y, hand_x, hand_y):
    sleeve_pts = capsule_pts(sh_x, sh_y, elb_x, elb_y, 2) | capsule_pts(elb_x, elb_y, hand_x, hand_y, 2)
    hand_pts = capsule_pts(hand_x, hand_y, hand_x + 1, hand_y - 1, 1)
    all_pts = sleeve_pts | hand_pts

    for px, py in all_pts:
        for ody in (-1, 0, 1):
            for odx in (-1, 0, 1):
                if (px + odx, py + ody) not in all_pts and py + ody > sh_y:
                    put_px(arr, px + odx, py + ody, OUTLINE)

    for px, py in sleeve_pts:
        col = PARKA_LIGHT if py <= elb_y and px >= elb_x else (PARKA_DARK if px < sh_x - 1 else PARKA)
        put_px(arr, px, py, col)
    for px, py in hand_pts:
        put_px(arr, px, py, SKIN)


def prep_torso_base(base_im, cut_y, swap_arm=True):
    arr = np.array(base_im)
    arr[cut_y:, :, :] = 0
    if swap_arm:
        arr[28:46, 44:, :] = 0
        for y in range(34, 44):
            for x in range(17, 25):
                rr, gg, bb, aa = arr[y, x]
                if aa > 128 and rr > 170 and gg > 130 and bb > 110:
                    arr[y, x] = PARKA_DARK if x < 21 else PARKA
    return arr


def reinforce_shorts(arr, y_top, x0, x1):
    for y in range(y_top, y_top + 3):
        for x in range(x0, x1 + 1):
            put_px(arr, x, y, OUTLINE if (y == y_top + 2 or x == x0 or x == x1) else (36, 34, 46, 255))


def build_4_run_frames(bases):
    """Compact 4-frame run loop using pure native frames with vertical head bob & alternating stride."""
    return [
        bases["contact"],  # 0: Contact (extended stride)
        bases["down_r"],   # 1: Down R (recoil, head drops +4px)
        bases["air"],      # 2: Air (push-off, head lifts -3px)
        bases["down_l"],   # 3: Down L (opposite knee/arm crouch)
    ]


def soften_laptop_glow_cone(arr):
    """Remove noisy floating green light wedge and keep only a subtle clean screen-edge glow."""
    for y in range(22, 45):
        for x in range(42, 57):
            rr, gg, bb, aa = [int(v) for v in arr[y, x]]
            if aa > 0 and gg > rr + 16 and gg > 115:
                arr[y, x] = (0, 0, 0, 0)

    # Subtle 2px glow strictly adjacent to laptop screen (x=48..50, y=38..42)
    for y in range(38, 43):
        if arr[y, 49, 3] == 0:
            arr[y, 49] = (80, 255, 80, 95)
        if arr[y, 48, 3] == 0 and y in (39, 40, 41):
            arr[y, 48] = (68, 255, 68, 45)


def draw_computer_desk(arr):
    """Draw a sleek dark pixel-art computer table supporting Futaba's laptop down to foot_y=65."""
    DESK_TOP = (86, 92, 112, 255)
    DESK_MID = (56, 60, 76, 255)
    DESK_DARK = (38, 40, 52, 255)

    # Tabletop under the laptop (y=47..50, x=41..61)
    for x in range(41, 62):
        put_px(arr, x, 46, OUTLINE)
        put_px(arr, x, 47, DESK_TOP)
        put_px(arr, x, 48, DESK_MID)
        put_px(arr, x, 49, DESK_DARK)
        put_px(arr, x, 50, OUTLINE)
    # Left/right end caps
    for y in range(47, 50):
        put_px(arr, 40, y, OUTLINE)
        put_px(arr, 62, y, OUTLINE)
    # Tiny glowing green power LED / accent on front edge of desk
    put_px(arr, 59, 48, ACID_LIME)

    # Helper for both left & right desk legs grounded at foot_y=65
    def draw_desk_leg(x_left, bracket_dx):
        for y in range(51, 65):
            put_px(arr, x_left, y, OUTLINE)
            put_px(arr, x_left + 1, y, DESK_MID)
            put_px(arr, x_left + 2, y, DESK_DARK)
            put_px(arr, x_left + 3, y, OUTLINE)
        for x in range(x_left - 2, x_left + 5):
            put_px(arr, x, 64, DESK_DARK if x_left - 1 <= x <= x_left + 3 else OUTLINE)
            put_px(arr, x, 65, OUTLINE)
        bx = x_left + bracket_dx
        put_px(arr, bx, 51, OUTLINE)
        put_px(arr, bx + (1 if bracket_dx < 0 else -1), 51, DESK_DARK)
        put_px(arr, bx + (1 if bracket_dx < 0 else -1), 52, OUTLINE)

    # Left desk leg (x=42..45) and right desk leg (x=56..59)
    draw_desk_leg(42, 4)
    draw_desk_leg(56, -2)


def build_chair_gremlin_8frames(chair_base_im):
    """Build the trimmer 8-frame Chair Gremlin loop with computer desk, dithered glow & LED pulse."""
    base_arr = np.array(chair_base_im)
    soften_laptop_glow_cone(base_arr)
    draw_computer_desk(base_arr)

    frames = []
    # (head_dy, cup_dy, ahoge_mode, hand_mode, glasses_mode, scr_shift, face_flash)
    spec = [
        (1, 0, "left", "l_down", "glow", 0, False),       # Frame 1
        (0, -1, "left", "r_down", "shift", 1, False),     # Frame 2
        (0, 0, "up", "double_tap", "solid", 2, False),    # Frame 3: opaque neon glasses + glare
        (0, 0, "right", "l_down", "flicker", 3, False),   # Frame 4
        (0, 1, "right", "enter_slam", "solid", 4, True),  # Frame 5: Enter slam + face/bangs flash
        (0, 0, "right", "hover", "glow", 5, False),       # Frame 6: hands hover + rapid scroll
        (0, 0, "center", "adjust", "glow", 6, False),     # Frame 7: subtle glasses-adjust + grin
        (0, 0, "center", "l_down", "glow", 7, False),     # Frame 8: reset to center
    ]

    for (h_dy, cup_dy, ahoge_mode, hand_mode, glasses_mode, scr_shift, face_flash) in spec:
        arr = base_arr.copy()

        # 1. Head + hair + headphone arc bob (y=11..34, x=20..46)
        if h_dy != 0:
            head_patch = arr[11:34, 20:46].copy()
            arr[11:34, 20:46] = 0
            arr[11 + h_dy : 34 + h_dy, 20:46] = head_patch

        # 2. Clean headphone earcup bounce + glowing orange/acid-green LED ring
        if cup_dy != 0:
            cup_patch = arr[25 + h_dy : 32 + h_dy, 23:30].copy()
            arr[25 + h_dy : 32 + h_dy, 23:30] = 0
            cy0 = 25 + h_dy + cup_dy
            arr[cy0 : cy0 + 7, 23:30] = cup_patch
            for ox in range(24, 29):
                put_px(arr, ox, cy0, OUTLINE)
                put_px(arr, ox, cy0 + 6, OUTLINE)
        led_col = ACID_LIME if glasses_mode == "solid" else (255, 130, 32, 255)
        led_y = 28 + h_dy + cup_dy
        put_px(arr, 26, led_y, led_col)
        put_px(arr, 27, led_y, led_col)

        # 3. Ahoge tip tilt (above headphone band y=10..14)
        if ahoge_mode == "up":
            put_px(arr, 40, 10 + h_dy, HAIR_ORANGE)
            put_px(arr, 41, 10 + h_dy, HAIR_LIGHT)
            put_px(arr, 40, 9 + h_dy, OUTLINE)
        elif ahoge_mode == "right":
            put_px(arr, 44, 12 + h_dy, HAIR_ORANGE)
            put_px(arr, 45, 13 + h_dy, HAIR_LIGHT)
        elif ahoge_mode == "left":
            put_px(arr, 37, 11 + h_dy, HAIR_LIGHT)
            put_px(arr, 36, 12 + h_dy, HAIR_ORANGE)

        # 4. Glasses glow & solid opaque neon-green flash with 1px diagonal white glare
        l_lens = [(x, y) for y in range(28 + h_dy, 31 + h_dy) for x in range(36, 39)]
        r_lens = [(x, y) for y in range(28 + h_dy, 31 + h_dy) for x in range(41, 44)]
        if glasses_mode == "solid":
            for gx, gy in l_lens + r_lens:
                put_px(arr, gx, gy, NEON_GREEN if (gx + gy) % 2 == 0 else ACID_LIME)
            for i in range(2):
                put_px(arr, 36 + i, 28 + h_dy + i, WHITE_GLARE)
                put_px(arr, 41 + i, 28 + h_dy + i, WHITE_GLARE)
        elif glasses_mode in ("shift", "flicker"):
            put_px(arr, 37, 29 + h_dy, ACID_LIME)
            put_px(arr, 42, 29 + h_dy, ACID_LIME)

        # 5. Acid-green rim light across bangs/nose on Enter slam
        if face_flash:
            for fx in range(34, 44):
                rr, gg, bb, aa = arr[25, fx]
                if aa > 128 and rr > 150:
                    put_px(arr, fx, 25, ACID_LIME)
            put_px(arr, 41, 30, ACID_LIME)
            put_px(arr, 42, 31, NEON_GREEN)

        # 6. Alternating typing hands above desk/laptop (x=42..49, y=40..44)
        def draw_hand(hx, hy, raised=False):
            if raised:
                put_px(arr, hx - 1, hy + 1, OUTLINE)
                put_px(arr, hx, hy + 1, PARKA_LIGHT)
                put_px(arr, hx + 1, hy + 1, OUTLINE)
            put_px(arr, hx, hy - 1, OUTLINE)
            put_px(arr, hx + 1, hy - 1, OUTLINE)
            put_px(arr, hx, hy, SKIN)
            put_px(arr, hx + 1, hy, SKIN)
            put_px(arr, hx, hy + 1, SKIN_SHADE if not raised else PARKA_LIGHT)

        if hand_mode == "l_down":
            draw_hand(42, 43, raised=False)
            draw_hand(46, 40, raised=True)
        elif hand_mode == "r_down":
            draw_hand(42, 40, raised=True)
            draw_hand(46, 43, raised=False)
        elif hand_mode == "double_tap":
            draw_hand(42, 43, raised=False)
            draw_hand(46, 43, raised=False)
            put_px(arr, 42, 40, (254, 218, 186, 130))
            put_px(arr, 42, 41, (254, 218, 186, 190))
            put_px(arr, 46, 40, (254, 218, 186, 130))
            put_px(arr, 46, 41, (254, 218, 186, 190))
        elif hand_mode == "enter_slam":
            draw_hand(42, 41, raised=True)
            for kx in range(45, 50):
                put_px(arr, kx, 43, PARKA_LIGHT if kx < 47 else SKIN)
                put_px(arr, kx, 42, OUTLINE)
            put_px(arr, 49, 43, ACID_LIME)
            put_px(arr, 49, 41, WHITE_GLARE)
        elif hand_mode == "hover":
            draw_hand(42, 39, raised=True)
            draw_hand(46, 39, raised=True)
        elif hand_mode == "adjust":
            draw_hand(40, 32, raised=False)
            put_px(arr, 40, 29, WHITE_GLARE)
            put_px(arr, 40, 31, OUTLINE)
            draw_hand(46, 42, raised=False)

        # 7. Scrolling 1px code lines on tiny laptop monitor (x=50..54, y=38..43)
        for row_i in range(5):
            sy = 38 + row_i
            put_px(arr, 50, sy, LAPTOP_DARK)
            put_px(arr, 51, sy, LAPTOP_DARK)
            put_px(arr, 52, sy, LAPTOP_DARK)
            if (row_i + scr_shift) % 2 == 0:
                put_px(arr, 50, sy, NEON_GREEN)
                put_px(arr, 51, sy, ACID_LIME if (row_i + scr_shift) % 4 == 0 else NEON_GREEN)

        frames.append(Image.fromarray(arr, "RGBA"))
    return frames


def build_caught_frames(raw_caught_cell, ref_h):
    """Build 4-frame Freeze -> Deep Hide -> Shiver -> Slow Peek behind stationary chair."""
    caught_base = p.normalize_cell(raw_caught_cell, ref_h * 1.12)
    base_arr = np.array(caught_base)

    # Clean away the weird floating laptop-corner flap on right (x>=46, y=35..54) & stray green specks
    base_arr[35:55, 46:] = 0
    for y in range(base_arr.shape[0]):
        for x in range(base_arr.shape[1]):
            rr, gg, bb, aa = [int(v) for v in base_arr[y, x]]
            if aa > 0 and gg > rr + 25 and gg > 120:
                base_arr[y, x] = (0, 0, 0, 0)

    # Reinforce clean right outline of chair backrest at x=45, y=34..48
    for y in range(34, 48):
        put_px(base_arr, 45, y, OUTLINE)

    # Extract stationary foreground chair backrest + seat + wheels (foot_y=65)
    chair_fg = np.zeros_like(base_arr)
    chair_fg[33:66, 28:47] = base_arr[33:66, 28:47]
    chair_fg[48:66, 14:47] = base_arr[48:66, 14:47]

    # Upper Futaba head/shoulders only (y<48 so legs/knees never poke below chair seat!)
    futaba_upper = base_arr.copy()
    futaba_upper[48:, :] = 0

    def compose_duck(dx, dy, sweat=False):
        out = np.zeros_like(base_arr)
        # Paste upper Futaba shifted behind stationary chair, clipped cleanly above y=48
        for sy in range(0, 48):
            ty = sy + dy
            if not (0 <= ty < 48):
                continue
            for sx in range(0, TARGET):
                tx = sx + dx
                if 0 <= tx < TARGET and futaba_upper[sy, sx, 3] > 0:
                    out[ty, tx] = futaba_upper[sy, sx]

        # Composite stationary foreground chair + tucked legs/wheels on top
        fg_mask = chair_fg[:, :, 3] > 0
        out[fg_mask] = chair_fg[fg_mask]
        if sweat:
            sx, sy = 20 + dx, 23 + dy
            put_px(out, sx, sy, OUTLINE)
            put_px(out, sx, sy + 1, (104, 232, 255, 255))
            put_px(out, sx, sy + 2, (180, 248, 255, 255))
        return Image.fromarray(out, "RGBA")

    # 0: Startle freeze (upright at matched scale + sweat drop)
    caught_0 = compose_duck(0, 0, sweat=True)
    # 1: Deep hide behind stationary chair (only hair, ahoge & top glasses rim peek!)
    caught_1 = compose_duck(0, 5, sweat=True)
    # 2: Deep hide nervous 1px tremble
    caught_2 = compose_duck(1, 5, sweat=False)
    # 3: Slow cautious halfway peek
    caught_3 = compose_duck(0, 2, sweat=False)
    return [caught_0, caught_1, caught_2, caught_3]


def build_gremlin_easter_eggs():
    raw_cells, heights = extract_grid_cells(GREMLIN_SHEET, 2, 2)
    ref_h = float(heights[0])
    # Slimmer Chair Gremlin scale (~55px tall, head_w=29px) to match idle_0 & snack_0 cleanly
    chair_base = p.normalize_cell(raw_cells[0], ref_h * 1.12)
    # Scale snack slightly tighter so her seated head/body matches chair_0 and idle_0 proportions
    snack_base = p.normalize_cell(raw_cells[1], ref_h * 1.06)
    # Compact panicked curled ball (~32x48) so it doesn't feel oversized when dragged
    panic_base = p.normalize_cell(raw_cells[3], ref_h * 1.24)

    chair_frames = build_chair_gremlin_8frames(chair_base)

    # 3-frame Yakisoba slurp animation (chair & boots stay planted on the ground!)
    snack_0 = snack_base

    # Frame 1: head nods slightly down (+1px) while chopsticks lift noodles up
    arr_s1 = np.array(snack_base)
    head_s = arr_s1[4:30, 18:52].copy()
    arr_s1[4:30, 18:52] = 0
    arr_s1[5:31, 18:52] = head_s
    noodle_p = arr_s1[32:39, 35:46].copy()
    arr_s1[32:39, 35:46] = 0
    arr_s1[31:38, 35:46] = noodle_p
    snack_1 = Image.fromarray(arr_s1, "RGBA")

    # Frame 2: head back upright, chopsticks dipped slightly down, happy slurp blush
    arr_s2 = np.array(snack_base)
    chop_p = arr_s2[33:39, 35:46].copy()
    arr_s2[33:39, 35:46] = 0
    arr_s2[34:40, 35:46] = chop_p
    put_px(arr_s2, 39, 26, (255, 148, 168, 255))
    put_px(arr_s2, 46, 26, (255, 148, 168, 255))
    snack_2 = Image.fromarray(arr_s2, "RGBA")

    caught_frames = build_caught_frames(raw_cells[2], ref_h)

    return chair_frames, [snack_0, snack_1, snack_2], caught_frames, panic_base


def build_code_cheer_frames(raw_code_cell, ref_h):
    """Build slimmer (36x58) teeth-free code cheer frames with stationary boots."""
    code_0_im = p.normalize_cell(raw_code_cell, ref_h * 1.10)
    arr0 = np.array(code_0_im)
    # Replace bright white teeth pixels in mouth region (y=30..33, x=32..38) with warm pink smile
    for y in range(30, 34):
        for x in range(32, 39):
            r, g, b, a = [int(v) for v in arr0[y, x]]
            if a > 128 and r > 210 and g > 205 and b > 205:
                arr0[y, x] = (232, 124, 136, 255) if y == 31 else (196, 94, 106, 255)
    code_0 = Image.fromarray(arr0, "RGBA")

    # Frame 1: boots stay anchored on floor (foot_y=65); only raised fist, ahoge & glasses sparkle pulse
    arr1 = arr0.copy()
    fist = arr1[18:36, 44:53].copy()
    arr1[18:36, 44:53] = 0
    arr1[17:35, 44:53] = fist
    # Subtle 1px ahoge pop & bright acid-green glasses glint
    put_px(arr1, 35, 9, HAIR_LIGHT)
    put_px(arr1, 30, 26, ACID_LIME)
    put_px(arr1, 30, 28, ACID_LIME)
    put_px(arr1, 29, 27, WHITE_GLARE)
    put_px(arr1, 31, 27, WHITE_GLARE)
    code_1 = Image.fromarray(arr1, "RGBA")
    return code_0, code_1


def build_fall_and_land_frames(raw_fall_cell, ref_h):
    """Build 2-frame flailing Fall animation and 2-frame Superhero Landing animation."""
    fall_0_im = p.normalize_cell(raw_fall_cell, ref_h * 1.14)
    arr_f0 = np.array(fall_0_im)
    # Subtle upward air-rush lines on fall_0
    for wy in (16, 17, 18, 28, 29):
        put_px(arr_f0, 12, wy, (200, 232, 255, 160))
        put_px(arr_f0, 54, wy - 2, (200, 232, 255, 160))
    fall_0 = Image.fromarray(arr_f0, "RGBA")

    # Frame 1: alternate flail bob & shifted wind streaks
    arr_f1 = np.array(p.shift_frame(fall_0_im, dy=-1))
    for wy in (20, 21, 22, 33, 34):
        put_px(arr_f1, 14, wy, (200, 232, 255, 160))
        put_px(arr_f1, 52, wy - 3, (200, 232, 255, 160))
    fall_1 = Image.fromarray(arr_f1, "RGBA")

    # Superhero Landing 0 (3-point impact crouch) + Landing 1 (glasses-adjust hero recovery)
    raw_land, land_heights = extract_grid_cells(LAND_SHEET, 1, 2)
    land_ref_h = float(land_heights[1]) * 1.16
    land_0_im = p.normalize_cell(raw_land[0], land_ref_h)
    arr_l0 = np.array(land_0_im)
    # Crisp shockwave dust puffs & impact lines flanking her fist & boots on foot_y=65
    ys, xs = np.where(arr_l0[:, :, 3] > 128)
    x_min, x_max = int(xs.min()), int(xs.max())
    for dx in (x_min - 3, x_min - 2, x_max + 2, x_max + 3):
        put_px(arr_l0, dx, 65, (215, 224, 240, 200))
    put_px(arr_l0, x_min - 4, 64, (215, 224, 240, 140))
    put_px(arr_l0, x_max + 4, 64, (215, 224, 240, 140))
    put_px(arr_l0, x_min - 1, 63, (235, 245, 255, 170))
    put_px(arr_l0, x_max + 1, 63, (235, 245, 255, 170))
    land_0 = Image.fromarray(arr_l0, "RGBA")

    land_1 = p.normalize_cell(raw_land[1], land_ref_h)
    return [fall_0, fall_1], [land_0, land_1]


def build_backseat_gamer_4frames():
    """Build 4-frame Backseat Co-Pilot gamepad-mashing loop grounded at foot_y=65."""
    raw_gamer, g_heights = extract_grid_cells(GAMER_SHEET, 1, 2)
    ref_h = float(g_heights[1]) * 1.08

    g0_im = p.normalize_cell(raw_gamer[0], ref_h)
    # Flip right sprite so both gamer poses face front-right consistently
    g1_flipped = np.fliplr(raw_gamer[1])
    g2_im = p.normalize_cell(g1_flipped, ref_h)

    # Subtle glowing controller LEDs & button-mash sparks on frames 1 and 3
    arr0 = np.array(g0_im)
    put_px(arr0, 35, 49, ACID_LIME)

    arr1 = np.array(g0_im)
    ctrl_box = arr1[44:55, 29:44].copy()
    arr1[44:55, 29:44] = 0
    arr1[43:54, 30:45] = ctrl_box
    put_px(arr1, 36, 47, ACID_LIME)
    put_px(arr1, 41, 44, (255, 168, 48, 220))
    put_px(arr1, 43, 46, WHITE_GLARE)

    arr2 = np.array(g2_im)
    put_px(arr2, 37, 48, ACID_LIME)

    arr3 = np.array(g2_im)
    ctrl_box2 = arr3[43:54, 31:46].copy()
    arr3[43:54, 31:46] = 0
    arr3[42:53, 30:45] = ctrl_box2
    put_px(arr3, 36, 46, NEON_GREEN)
    put_px(arr3, 32, 43, (120, 232, 255, 220))
    put_px(arr3, 41, 42, (255, 168, 48, 220))

    return [
        Image.fromarray(arr0, "RGBA"),
        Image.fromarray(arr1, "RGBA"),
        Image.fromarray(arr2, "RGBA"),
        Image.fromarray(arr3, "RGBA"),
    ]


def main():
    base_cells, raw_base_cells, base_ref_h = extract_base_poses()
    run_bases = extract_run_bases()
    run_frames = build_4_run_frames(run_bases)
    chair_frames, snack_frames, caught_frames, panic_ball = build_gremlin_easter_eggs()
    code_0, code_1 = build_code_cheer_frames(raw_base_cells[8], base_ref_h)
    fall_frames, land_frames = build_fall_and_land_frames(raw_base_cells[10], base_ref_h)
    gamer_frames = build_backseat_gamer_4frames()

    frames = [
        ("idle_0", base_cells[0]),
        ("idle_1", p.shift_frame(base_cells[0], dy=1)),
        ("blink", base_cells[1]),
        ("walk_0", run_frames[0]),
        ("walk_1", run_frames[1]),
        ("walk_2", run_frames[2]),
        ("walk_3", run_frames[3]),
        ("sit_0", base_cells[5]),
        ("sit_1", base_cells[6]),
        ("vibe_0", base_cells[7]),
        ("vibe_1", p.shift_frame(base_cells[7].transpose(Image.Transpose.FLIP_LEFT_RIGHT), dy=-1)),
        ("code_0", code_0),
        ("code_1", code_1),
        ("gamer_0", gamer_frames[0]),
        ("gamer_1", gamer_frames[1]),
        ("gamer_2", gamer_frames[2]),
        ("gamer_3", gamer_frames[3]),
        ("chair_0", chair_frames[0]),
        ("chair_1", chair_frames[1]),
        ("chair_2", chair_frames[2]),
        ("chair_3", chair_frames[3]),
        ("chair_4", chair_frames[4]),
        ("chair_5", chair_frames[5]),
        ("chair_6", chair_frames[6]),
        ("chair_7", chair_frames[7]),
        ("snack_0", snack_frames[0]),
        ("snack_1", snack_frames[1]),
        ("snack_2", snack_frames[2]),
        ("caught_0", caught_frames[0]),
        ("caught_1", caught_frames[1]),
        ("caught_2", caught_frames[2]),
        ("caught_3", caught_frames[3]),
        ("fall_0", fall_frames[0]),
        ("fall_1", fall_frames[1]),
        ("land_0", land_frames[0]),
        ("land_1", land_frames[1]),
        ("sleep_0", base_cells[9]),
        ("sleep_1", p.shift_frame(base_cells[9], dy=1)),
        ("dragged", panic_ball),
        ("happy", base_cells[11]),
        ("peek", caught_frames[3]),
    ]

    out_png = os.path.join(OUT_DIR, "futaba.png")
    out_meta = os.path.join(OUT_DIR, "futaba.json")
    sheet = Image.new("RGBA", (TARGET * len(frames), TARGET), (0, 0, 0, 0))
    for i, (_, fim) in enumerate(frames):
        sheet.paste(fim, (i * TARGET, 0))
    sheet.save(out_png)
    with open(out_meta, "w") as f:
        json.dump(
            {
                "frame_width": TARGET,
                "frame_height": TARGET,
                "frames": {name: i for i, (name, _) in enumerate(frames)},
            },
            f,
            indent=2,
        )

    scratch = "/home/faewon/.gemini/antigravity-cli/brain/4532fa2f-dc2e-4f0f-9409-ce14f8ea7fee/scratch"
    os.makedirs(scratch, exist_ok=True)

    # Preview 8-frame Chair Gremlin + Snack + 4-stage Caught (13 frames in 7x2)
    prev_list = chair_frames + [snack_frames[0]] + caught_frames + [panic_ball]
    prev = Image.new("RGBA", (TARGET * 7 * 2, TARGET * 2 * 2), (28, 26, 38, 255))
    for i, fim in enumerate(prev_list):
        col = i % 7
        row = i // 7
        big = fim.resize((TARGET * 2, TARGET * 2), Image.Resampling.NEAREST)
        prev.paste(big, (col * TARGET * 2, row * TARGET * 2), big)
    prev_path = os.path.join(scratch, "futaba_gremlin_preview.png")
    prev.save(prev_path)
    print(f"Saved {out_png} ({sheet.size[0]}x{sheet.size[1]}, {len(frames)} frames)")
    print(f"Saved preview {prev_path}")


if __name__ == "__main__":
    main()
