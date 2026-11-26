#!/usr/bin/env python3
"""Generate a crisp 40x44 pixel-art anime girl sprite sheet (assets/sprites.png)."""

import json
import os
import struct
import zlib

FRAME_W = 40
FRAME_H = 44

PALETTE = {
    ".": (0, 0, 0, 0),
    # Outlines
    "O": (44, 26, 54, 255),      # Deep plum outline
    "k": (122, 68, 66, 255),     # Warm skin outline
    # Skin
    "S": (255, 226, 206, 255),   # Fair peach skin
    "s": (240, 186, 166, 255),   # Skin shadow
    "b": (255, 146, 168, 255),   # Cheek blush
    # Hair (Sakura pink)
    "h": (255, 210, 226, 255),   # Bright pastel highlight
    "H": (255, 152, 188, 255),   # Base sakura pink
    "d": (218, 102, 148, 255),   # Mid rose shadow
    "D": (166, 66, 114, 255),    # Deep berry shadow
    # Eyes & whites
    "W": (255, 255, 255, 255),   # Pure white (shine, sclera, socks, collar)
    "w": (214, 220, 240, 255),   # Cool white shadow
    "E": (76, 194, 246, 255),    # Bright sky-cyan iris
    "e": (36, 70, 142, 255),     # Deep indigo upper iris / pupil
    # Outfit (Dark navy cardigan + crimson ribbon + pleated skirt)
    "c": (88, 96, 142, 255),     # Cardigan highlight
    "C": (58, 62, 98, 255),      # Cardigan base
    "K": (42, 40, 68, 255),      # Skirt / shoes dark
    "R": (238, 72, 104, 255),    # Cherry ribbon
    "r": (174, 42, 72, 255),     # Ribbon shadow
    # Accents (Gold star clip & cyan headphones/glasses)
    "G": (255, 218, 92, 255),    # Gold highlight
    "g": (216, 156, 50, 255),    # Gold shadow
    "P": (118, 234, 252, 255),   # Cyan glow (headphones / glasses / sparkle)
    "p": (64, 152, 210, 255),    # Cyan shade
}


def blank_canvas():
    return [["." for _ in range(FRAME_W)] for _ in range(FRAME_H)]


def put(grid, x, y, ch):
    if 0 <= x < FRAME_W and 0 <= y < FRAME_H and ch != ".":
        grid[y][x] = ch


def hline(grid, x0, x1, y, ch):
    for x in range(x0, x1 + 1):
        put(grid, x, y, ch)


def paint_rows(grid, ox, oy, rows):
    for dy, row in enumerate(rows):
        for dx, ch in enumerate(row):
            if ch != ".":
                put(grid, ox + dx, oy + dy, ch)


def draw_twintails(grid, dy=0, swing_l=0, swing_r=0, lift=0):
    """Draw sakura twin-tails behind head/body."""
    # Left tail anchor around (10, 11+dy), right tail around (29, 11+dy)
    ly = 11 + dy - lift
    ry = 11 + dy - lift
    # Left twin-tail
    left_tail = [
        "....OO...",
        "...OHHO..",
        "..OhHHdO.",
        ".OhHHdDO.",
        ".OHHdDO..",
        "OHHdDO...",
        "OHdDO....",
        "OdDO.....",
        ".OO......",
    ]
    # Right twin-tail (mirrored)
    right_tail = [
        "...OO....",
        "..OHHO...",
        ".OdHHhO..",
        ".ODdHHhO.",
        "..ODdHHO.",
        "...ODdHHO",
        "....ODdHO",
        ".....ODdO",
        "......OO.",
    ]
    for row_i, row in enumerate(left_tail):
        shift = (swing_l * row_i) // 5
        for col_i, ch in enumerate(row):
            if ch != ".":
                put(grid, 3 + col_i + shift, ly + row_i, ch)
    for row_i, row in enumerate(right_tail):
        shift = (swing_r * row_i) // 5
        for col_i, ch in enumerate(row):
            if ch != ".":
                put(grid, 28 + col_i + shift, ry + row_i, ch)

    # Hair tie ribbons
    paint_rows(grid, 9, 10 + dy, ["ORR", "OrO"])
    paint_rows(grid, 28, 10 + dy, ["RRO", "OrO"])


def draw_head(grid, dy=0, eye_mode="normal", mouth="smile", ahoge_tilt=0, headphones=False, glasses=False):
    """Draw chibi anime head, hair dome, bangs, and expression."""
    # Back hair & dome (y=4..19 + dy)
    dome = [
        "......OOOOOOOO......",
        "....OOHHHHHHHHOO....",
        "...OHHhHHHHHHhHHO...",
        "..OHHhHHHHHHHHhHHO..",
        ".OHHHHHHHHHHHHHHHHO.",
        ".OHHhHHhHHHHhHHhHHO.",
        "OHHHdHHHHHHHHHHdHHHO",
        "OHHd.dHHHHHHHHd.dHHO",
        "OHd...dHHHHHHd...dHO",
        "OHd..............dHO",
        "OHd..............dHO",
        "OHd..............dHO",
        "Od................dO",
        "Od................dO",
        ".O................O.",
    ]
    paint_rows(grid, 10, 5 + dy, dome)

    # Ahoge (cute cowlick at top)
    if ahoge_tilt == 0:
        paint_rows(grid, 18, 2 + dy, [".OO.", "OHhO", ".OO."])
    elif ahoge_tilt < 0:
        paint_rows(grid, 17, 2 + dy, ["OO..", "OhO.", ".OO."])
    else:
        paint_rows(grid, 19, 2 + dy, ["..OO", ".OhO", ".OO."])

    # Face skin interior (x=13..26, y=12..19 + dy)
    for y in range(12, 19):
        hline(grid, 13, 26, y + dy, "S")
    hline(grid, 14, 25, 19 + dy, "S")
    # Chin & cheek outline
    put(grid, 12, 16 + dy, "k")
    put(grid, 12, 17 + dy, "k")
    put(grid, 13, 18 + dy, "k")
    put(grid, 27, 16 + dy, "k")
    put(grid, 27, 17 + dy, "k")
    put(grid, 26, 18 + dy, "k")
    hline(grid, 14, 25, 20 + dy, "k")
    # Subtle forehead shadow under bangs
    hline(grid, 14, 25, 12 + dy, "s")

    # Bangs & side locks overlapping top of forehead
    bangs = [
        "..OOHHHHHHHHHHHHOO..",
        ".OHhHHhHHHHHHhHHhHO.",
        "OHHHdHHdHHHHdHHdHHHO",
        "OHd..dH.dHHd.Hd..dHO",
        "Od....d..dd..d....dO",
    ]
    paint_rows(grid, 10, 9 + dy, bangs)

    # Gold star hairclip on viewer's left bang
    paint_rows(grid, 12, 10 + dy, ["GG", "gO"])

    # Side hair framing cheeks
    for y in range(13, 18):
        put(grid, 11, y + dy, "H")
        put(grid, 12, y + dy, "d" if y < 17 else "O")
        put(grid, 28, y + dy, "H")
        put(grid, 27, y + dy, "d" if y < 17 else "O")
    put(grid, 11, 18 + dy, "O")
    put(grid, 28, 18 + dy, "O")

    # Cheek blush
    put(grid, 13, 16 + dy, "b")
    put(grid, 14, 17 + dy, "b")
    put(grid, 26, 16 + dy, "b")
    put(grid, 25, 17 + dy, "b")

    # Eyes (left x=14..17, right x=22..25, y=13..16 + dy)
    if eye_mode == "normal":
        paint_rows(grid, 14, 13 + dy, [
            "OOOO",
            "OWee",
            "WEeE",
            "WEEw",
        ])
        paint_rows(grid, 22, 13 + dy, [
            "OOOO",
            "eWeO",
            "EeEW",
            "wEEW",
        ])
    elif eye_mode == "happy":
        # Cute ^ ^ curved eyes
        paint_rows(grid, 14, 14 + dy, [
            ".OO.",
            "O..O",
        ])
        paint_rows(grid, 22, 14 + dy, [
            ".OO.",
            "O..O",
        ])
    elif eye_mode == "blink":
        hline(grid, 14, 17, 15 + dy, "O")
        hline(grid, 22, 25, 15 + dy, "O")
    elif eye_mode == "sleep":
        hline(grid, 14, 17, 15 + dy, "O")
        put(grid, 15, 16 + dy, "k")
        hline(grid, 22, 25, 15 + dy, "O")
        put(grid, 24, 16 + dy, "k")
    elif eye_mode == "surprised":
        # > < cute flail eyes
        paint_rows(grid, 14, 14 + dy, [
            "O...",
            ".OO.",
            "O...",
        ])
        paint_rows(grid, 22, 14 + dy, [
            "...O",
            ".OO.",
            "...O",
        ])
    elif eye_mode == "peek":
        # Looking slightly up-right toward cursor
        paint_rows(grid, 14, 13 + dy, [
            "OOOO",
            "WeEO",
            "WEEw",
            "WWWW",
        ])
        paint_rows(grid, 22, 13 + dy, [
            "OOOO",
            "eEWO",
            "EEWw",
            "WWWW",
        ])

    # Mouth (x=19..20, y=18..19 + dy)
    if mouth == "smile":
        put(grid, 19, 18 + dy, "k")
        put(grid, 20, 18 + dy, "k")
    elif mouth == "open":
        paint_rows(grid, 19, 18 + dy, ["kk", "bb"])
    elif mouth == "cat":
        put(grid, 18, 18 + dy, "k")
        put(grid, 19, 19 + dy, "k")
        put(grid, 20, 19 + dy, "k")
        put(grid, 21, 18 + dy, "k")
    elif mouth == "yawn":
        paint_rows(grid, 19, 18 + dy, ["kk", "kb"])

    # Optional cute cyber-glasses (coding mode)
    if glasses:
        hline(grid, 13, 17, 14 + dy, "P")
        hline(grid, 18, 21, 14 + dy, "p")
        hline(grid, 22, 26, 14 + dy, "P")
        put(grid, 13, 15 + dy, "p")
        put(grid, 26, 15 + dy, "p")

    # Optional cozy headphones (music vibing mode)
    if headphones:
        # Headband arc
        hline(grid, 14, 25, 4 + dy, "P")
        put(grid, 12, 5 + dy, "p")
        put(grid, 13, 5 + dy, "P")
        put(grid, 26, 5 + dy, "P")
        put(grid, 27, 5 + dy, "p")
        # Ear cups
        paint_rows(grid, 9, 12 + dy, ["OO", "OP", "Op", "OO"])
        paint_rows(grid, 29, 12 + dy, ["OO", "PO", "pO", "OO"])


def draw_torso_and_skirt(grid, dy=0, sit=False):
    """Draw neck, white collar, crimson ribbon, navy cardigan, and pleated skirt."""
    # Neck
    hline(grid, 18, 21, 20 + dy, "s")
    put(grid, 17, 20 + dy, "k")
    put(grid, 22, 20 + dy, "k")

    # Cardigan + collar (y=21..29 + dy)
    torso = [
        "....OOOWWWWOOWWWWOOO....",
        "...OcCCWWRRWWRRWWCCcO...",
        "...OcCCWWRRRRRRWWCCcO...",
        "...OCCCCwOrRRrOwCCCCO...",
        "...OCCCCCrOOOOOrCCCCO...",
        "...OCCCCCCCCCCCCCCCCO...",
        "...OCCCCcCCCCCCcCCCCO...",
        "...OCCCCCCCCCCCCCCCCO...",
        "..OOKKKKKKKKKKKKKKKKOO..",
    ]
    paint_rows(grid, 8, 21 + dy, torso)

    # Pleated skirt (y=30..34 + dy)
    if sit:
        skirt = [
            ".OOKKKcKKKKKKcKKKKKKKOO.",
            "OKKKKKcKKKKKKcKKKKKKKKKO",
            "OKKKKKOKKKKKKOKKKKKKKKKO",
            ".OOOOOOOOOOOOOOOOOOOOOO.",
        ]
        paint_rows(grid, 8, 30 + dy, skirt)
    else:
        skirt = [
            "..OKKKcKKKKKKcKKKKKKKO..",
            ".OKKKKcKKKKKKcKKKKKKKKO.",
            ".OKKKKOKKKKKKOKKKKKKKKO.",
            "OKKKKKOKKKKKKOKKKKKKKKKO",
            ".OOOOOOOOOOOOOOOOOOOOOO.",
        ]
        paint_rows(grid, 8, 30 + dy, skirt)


def draw_arms(grid, dy=0, pose="idle"):
    """Draw left & right cardigan sleeves and cute hands."""
    if pose == "idle":
        paint_rows(grid, 9, 22 + dy, ["OcO", "OCO", "OCO", "OCO", "OSO", ".k."])
        paint_rows(grid, 28, 22 + dy, ["OcO", "OCO", "OCO", "OCO", "OSO", ".k."])
    elif pose == "walk_l":
        # Left arm forward, right arm back
        paint_rows(grid, 8, 22 + dy, [".OcO", "OCO.", "OCO.", "OSO.", ".k.."])
        paint_rows(grid, 28, 22 + dy, ["OcO.", ".OCO", ".OCO", ".OSO", "..k."])
    elif pose == "walk_r":
        paint_rows(grid, 8, 22 + dy, ["OcO.", ".OCO", ".OCO", ".OSO", "..k."])
        paint_rows(grid, 28, 22 + dy, [".OcO", "OCO.", "OCO.", "OSO.", ".k.."])
    elif pose == "raise_both":
        # Happy / vibe / dragged arms up!
        paint_rows(grid, 7, 18 + dy, [".k.", "OSO", "OCO", "OCO", ".OO"])
        paint_rows(grid, 30, 18 + dy, [".k.", "OSO", "OCO", "OCO", "OO."])
    elif pose == "cheer_r":
        paint_rows(grid, 9, 22 + dy, ["OcO", "OCO", "OCO", "OSO", ".k."])
        paint_rows(grid, 30, 17 + dy, [".k.", "OSO", "OCO", "OCO", "OO."])
    elif pose == "vibe_l":
        paint_rows(grid, 7, 19 + dy, [".k.", "OSO", "OCO", "OCO", ".OO"])
        paint_rows(grid, 29, 23 + dy, ["OcO", "OCO", "OCO", "OSO", ".k."])
    elif pose == "vibe_r":
        paint_rows(grid, 8, 23 + dy, ["OcO", "OCO", "OCO", "OSO", ".k."])
        paint_rows(grid, 30, 19 + dy, [".k.", "OSO", "OCO", "OCO", "OO."])
    elif pose == "sit":
        paint_rows(grid, 9, 22 + dy, ["OcO", "OCO", "OCO", "OCO", ".OS", "..k"])
        paint_rows(grid, 28, 22 + dy, ["OcO", "OCO", "OCO", "OCO", "SO.", "k.."])


def draw_legs(grid, dy=0, pose="idle"):
    """Draw legs, thigh-high socks, and Mary Jane shoes."""
    if pose == "idle":
        leg = [
            "kSSk",
            "kSSk",
            "OWWO",
            "OWwO",
            "OWwO",
            "OKKO",
            "OOOO",
        ]
        paint_rows(grid, 14, 35 + dy, leg)
        paint_rows(grid, 22, 35 + dy, leg)
    elif pose == "walk_1":
        # Left forward, right back
        paint_rows(grid, 12, 35 + dy, [".kSSk", "kSSk.", "OWWO.", "OWwO.", "OKKO.", "OOOO."])
        paint_rows(grid, 23, 35 + dy, ["kSSk.", ".kSSk", ".OWWO", ".OWwO", ".OKKO", ".OOOO"])
    elif pose == "walk_2":
        paint_rows(grid, 14, 35 + dy, ["kSSk", "OWWO", "OWwO", "OKKO", "OOOO"])
        paint_rows(grid, 22, 34 + dy, ["kSSk", "OWWO", "OKKO", "OOOO"])
    elif pose == "walk_3":
        paint_rows(grid, 13, 35 + dy, ["kSSk.", ".kSSk", ".OWWO", ".OWwO", ".OKKO", ".OOOO"])
        paint_rows(grid, 22, 35 + dy, [".kSSk", "kSSk.", "OWWO.", "OWwO.", "OKKO.", "OOOO."])
    elif pose == "sit_1":
        # Legs dangling forward over window edge
        paint_rows(grid, 13, 34 + dy, ["kSSk", "OWWO", "OWwO", "OKKO", "OOOO"])
        paint_rows(grid, 23, 34 + dy, ["kSSk", "OWWO", "OWwO", "OKKO", "OOOO"])
    elif pose == "sit_2":
        # Kicking feet happily!
        paint_rows(grid, 12, 34 + dy, [".kSSk", "OWWO.", "OKKO.", "OOOO."])
        paint_rows(grid, 23, 33 + dy, ["kSSk.", ".OWWO", ".OWwO", ".OKKO", ".OOOO"])
    elif pose == "flail":
        # Flailing while dragged
        paint_rows(grid, 11, 35 + dy, [".kSSk", "OWWO.", "OKKO.", "OOOO."])
        paint_rows(grid, 24, 35 + dy, ["kSSk.", ".OWWO", ".OKKO", ".OOOO"])
    elif pose == "sleep":
        # Curled cozy knees
        paint_rows(grid, 12, 34 + dy, ["..kSSk", ".OWWWO", "OKKKO.", "OOOO.."])
        paint_rows(grid, 22, 34 + dy, ["kSSk..", "OWWWO.", ".OKKKO", "..OOOO"])


def make_frame(
    dy=0,
    eye_mode="normal",
    mouth="smile",
    ahoge_tilt=0,
    swing_l=0,
    swing_r=0,
    lift=0,
    arm_pose="idle",
    leg_pose="idle",
    sit=False,
    headphones=False,
    glasses=False,
):
    g = blank_canvas()
    draw_twintails(g, dy=dy, swing_l=swing_l, swing_r=swing_r, lift=lift)
    draw_torso_and_skirt(g, dy=dy, sit=sit)
    draw_legs(g, dy=dy, pose=leg_pose)
    draw_arms(g, dy=dy, pose=arm_pose)
    draw_head(
        g,
        dy=dy,
        eye_mode=eye_mode,
        mouth=mouth,
        ahoge_tilt=ahoge_tilt,
        headphones=headphones,
        glasses=glasses,
    )
    return g


def write_png(path, width, height, rgba_bytes):
    def chunk(tag, data):
        return struct.pack("!I", len(data)) + tag + data + struct.pack("!I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    raw = bytearray()
    stride = width * 4
    for y in range(height):
        raw.append(0)  # filter type 0
        raw.extend(rgba_bytes[y * stride : (y + 1) * stride])

    ihdr = struct.pack("!IIBBBBB", width, height, 8, 6, 0, 0, 0)
    idat = zlib.compress(bytes(raw), 9)
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        f.write(chunk(b"IHDR", ihdr))
        f.write(chunk(b"IDAT", idat))
        f.write(chunk(b"IEND", b""))


def main():
    frames_spec = [
        ("idle_0", make_frame(dy=0, eye_mode="normal", mouth="cat", ahoge_tilt=0)),
        ("idle_1", make_frame(dy=1, eye_mode="normal", mouth="cat", ahoge_tilt=1, swing_l=-1, swing_r=1)),
        ("blink", make_frame(dy=0, eye_mode="blink", mouth="cat", ahoge_tilt=0)),
        ("walk_0", make_frame(dy=0, eye_mode="normal", mouth="smile", ahoge_tilt=-1, swing_l=-2, swing_r=1, arm_pose="walk_l", leg_pose="walk_1")),
        ("walk_1", make_frame(dy=-1, eye_mode="normal", mouth="smile", ahoge_tilt=0, swing_l=-1, swing_r=2, arm_pose="idle", leg_pose="walk_2")),
        ("walk_2", make_frame(dy=0, eye_mode="normal", mouth="smile", ahoge_tilt=1, swing_l=-1, swing_r=2, arm_pose="walk_r", leg_pose="walk_3")),
        ("walk_3", make_frame(dy=-1, eye_mode="normal", mouth="smile", ahoge_tilt=0, swing_l=-2, swing_r=1, arm_pose="idle", leg_pose="walk_2")),
        ("sit_0", make_frame(dy=2, eye_mode="normal", mouth="cat", sit=True, arm_pose="sit", leg_pose="sit_1")),
        ("sit_1", make_frame(dy=2, eye_mode="happy", mouth="open", ahoge_tilt=1, sit=True, arm_pose="sit", leg_pose="sit_2")),
        ("vibe_0", make_frame(dy=0, eye_mode="happy", mouth="open", ahoge_tilt=-1, swing_l=-3, swing_r=1, lift=1, arm_pose="vibe_l", leg_pose="walk_2", headphones=True)),
        ("vibe_1", make_frame(dy=-1, eye_mode="happy", mouth="cat", ahoge_tilt=1, swing_l=-1, swing_r=3, lift=1, arm_pose="vibe_r", leg_pose="idle", headphones=True)),
        ("code_0", make_frame(dy=0, eye_mode="normal", mouth="cat", ahoge_tilt=0, arm_pose="idle", leg_pose="idle", glasses=True)),
        ("code_1", make_frame(dy=-1, eye_mode="happy", mouth="open", ahoge_tilt=1, swing_l=-1, swing_r=2, arm_pose="cheer_r", leg_pose="idle", glasses=True)),
        ("sleep_0", make_frame(dy=3, eye_mode="sleep", mouth="yawn", ahoge_tilt=-1, sit=True, arm_pose="sit", leg_pose="sleep")),
        ("sleep_1", make_frame(dy=4, eye_mode="sleep", mouth="smile", ahoge_tilt=-1, sit=True, arm_pose="sit", leg_pose="sleep")),
        ("dragged", make_frame(dy=-1, eye_mode="surprised", mouth="open", ahoge_tilt=1, swing_l=-3, swing_r=3, lift=2, arm_pose="raise_both", leg_pose="flail")),
        ("happy", make_frame(dy=-1, eye_mode="happy", mouth="open", ahoge_tilt=1, swing_l=-2, swing_r=2, lift=1, arm_pose="raise_both", leg_pose="idle")),
        ("peek", make_frame(dy=0, eye_mode="peek", mouth="cat", ahoge_tilt=1, arm_pose="idle", leg_pose="idle")),
    ]

    num_frames = len(frames_spec)
    sheet_w = FRAME_W * num_frames
    sheet_h = FRAME_H
    buf = bytearray(sheet_w * sheet_h * 4)

    names = []
    for idx, (name, grid) in enumerate(frames_spec):
        names.append(name)
        x_off = idx * FRAME_W
        for y in range(FRAME_H):
            for x in range(FRAME_W):
                r, g, b, a = PALETTE[grid[y][x]]
                p = ((y * sheet_w) + (x_off + x)) * 4
                buf[p] = r
                buf[p + 1] = g
                buf[p + 2] = b
                buf[p + 3] = a

    out_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), "assets")
    os.makedirs(out_dir, exist_ok=True)
    png_path = os.path.join(out_dir, "sprites.png")
    meta_path = os.path.join(out_dir, "sprites.json")
    write_png(png_path, sheet_w, sheet_h, buf)
    with open(meta_path, "w") as f:
        json.dump(
            {
                "frame_width": FRAME_W,
                "frame_height": FRAME_H,
                "frames": {name: i for i, name in enumerate(names)},
            },
            f,
            indent=2,
        )

    print(f"Generated {png_path} ({sheet_w}x{sheet_h}, {num_frames} frames)")
    # Print preview of idle_0
    print("--- idle_0 ASCII preview ---")
    for row in frames_spec[0][1][2:42]:
        print("".join(" " if c == "." else c for c in row))


if __name__ == "__main__":
    main()
