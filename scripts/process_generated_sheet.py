#!/usr/bin/env python3
"""Slice, chroma-key, despill, align, and pack a 4x3 anime pixel sprite sheet."""

import argparse
import json
import os
from collections import deque
import numpy as np
from PIL import Image

DEFAULT_SRC = (
    "/home/faewon/.gemini/antigravity-cli/brain/"
    "4532fa2f-dc2e-4f0f-9409-ce14f8ea7fee/"
    "chibi_mascot_sprite_sheet_clean_1795661014383.jpg"
)
OUT_DIR = os.path.join(os.path.dirname(os.path.dirname(__file__)), "assets")
TARGET_W = 68
TARGET_H = 68


def remove_green_bg(rgb: np.ndarray) -> np.ndarray:
    """Chroma-key pure #00FF00 green background + pockets while preserving green clothes/glows."""
    h, w, _ = rgb.shape
    r = rgb[:, :, 0].astype(np.int32)
    g = rgb[:, :, 1].astype(np.int32)
    b = rgb[:, :, 2].astype(np.int32)

    green_excess = g - np.maximum(r, b)
    # Pure chroma-key green (safe against olive parka and bright mint/white sparkles)
    pure_bg = (green_excess > 120) & (g > 165) & (r < 85) & (b < 85)
    soft_bg = (green_excess > 26) & (g > 95) & (r < 100) & (b < 100)
    bg = pure_bg.copy()

    # Flood-fill soft_bg connected to borders or pure_bg
    q = deque()
    for y in range(h):
        for x in range(w):
            if bg[y, x]:
                q.append((y, x))
            elif (y == 0 or y == h - 1 or x == 0 or x == w - 1) and soft_bg[y, x]:
                bg[y, x] = True
                q.append((y, x))

    while q:
        cy, cx = q.popleft()
        for dy, dx in ((-1, 0), (1, 0), (0, -1), (0, 1)):
            ny, nx = cy + dy, cx + dx
            if 0 <= ny < h and 0 <= nx < w and not bg[ny, nx] and soft_bg[ny, nx]:
                bg[ny, nx] = True
                q.append((ny, nx))

    rgba = np.zeros((h, w, 4), dtype=np.uint8)
    rgba[:, :, :3] = rgb
    rgba[:, :, 3] = np.where(bg, 0, 255)

    # Despill green JPEG fringe on opaque pixels touching transparent background
    bg_dilated = bg.copy()
    bg_dilated[1:, :] |= bg[:-1, :]
    bg_dilated[:-1, :] |= bg[1:, :]
    bg_dilated[:, 1:] |= bg[:, :-1]
    bg_dilated[:, :-1] |= bg[:, 1:]
    ring2 = bg_dilated.copy()
    ring2[1:, :] |= bg_dilated[:-1, :]
    ring2[:-1, :] |= bg_dilated[1:, :]
    ring2[:, 1:] |= bg_dilated[:, :-1]
    ring2[:, :-1] |= bg_dilated[:, 1:]

    edge = (~bg) & ring2
    max_rb = np.maximum(r, b)
    # Only despill border pixels that aren't intentionally bright white-mint highlights
    despill_mask = edge & (g > max_rb + 6) & (r < 110)
    rgba[:, :, 1] = np.where(despill_mask, (max_rb + 4).clip(0, 255).astype(np.uint8), rgba[:, :, 1])

    opaque = rgba[:, :, 3] > 0
    neighbors = (
        np.roll(opaque, 1, 0).astype(int)
        + np.roll(opaque, -1, 0).astype(int)
        + np.roll(opaque, 1, 1).astype(int)
        + np.roll(opaque, -1, 1).astype(int)
    )
    rgba[opaque & (neighbors <= 1), 3] = 0
    return rgba


def find_spans(counts, min_gap=8):
    spans = []
    in_span = False
    start = 0
    for i, c in enumerate(counts):
        if c > 4 and not in_span:
            in_span = True
            start = i
        elif c <= 4 and in_span:
            in_span = False
            spans.append((start, i))
    if in_span:
        spans.append((start, len(counts)))
    merged = []
    for s, e in spans:
        if merged and s - merged[-1][1] < min_gap:
            merged[-1] = (merged[-1][0], e)
        else:
            merged.append((s, e))
    return merged


def normalize_cell(cell_rgba: np.ndarray, scale_ref_h: float) -> Image.Image:
    alpha = cell_rgba[:, :, 3] > 128
    ys, xs = np.where(alpha)
    y0, y1 = int(ys.min()), int(ys.max()) + 1
    x0, x1 = int(xs.min()), int(xs.max()) + 1
    crop = Image.fromarray(cell_rgba[y0:y1, x0:x1], "RGBA")

    target_char_h = max(16, int(round((y1 - y0) * (62.0 / scale_ref_h))))
    target_char_w = max(16, int(round((x1 - x0) * (62.0 / scale_ref_h))))
    if target_char_w > TARGET_W - 2:
        ratio = (TARGET_W - 2) / target_char_w
        target_char_w = TARGET_W - 2
        target_char_h = max(16, int(round(target_char_h * ratio)))
    if target_char_h > TARGET_H - 2:
        ratio = (TARGET_H - 2) / target_char_h
        target_char_h = TARGET_H - 2
        target_char_w = max(16, int(round(target_char_w * ratio)))

    resized = crop.resize((target_char_w, target_char_h), Image.Resampling.NEAREST)
    canvas = Image.new("RGBA", (TARGET_W, TARGET_H), (0, 0, 0, 0))
    px = (TARGET_W - target_char_w) // 2
    py = TARGET_H - 2 - target_char_h
    canvas.paste(resized, (px, py))
    return canvas


def shift_frame(im: Image.Image, dx: int = 0, dy: int = 0) -> Image.Image:
    canvas = Image.new("RGBA", im.size, (0, 0, 0, 0))
    canvas.paste(im, (dx, dy))
    return canvas


def process_sheet(src_path: str, out_png: str, meta_path: str | None, preview_path: str | None):
    rgb = np.array(Image.open(src_path).convert("RGB"))
    r = rgb[:, :, 0].astype(np.int32)
    g = rgb[:, :, 1].astype(np.int32)
    b = rgb[:, :, 2].astype(np.int32)
    fg = ~((g - np.maximum(r, b) > 50) & (g > 140))

    row_spans = find_spans(fg.sum(axis=1), min_gap=6)
    assert len(row_spans) == 3, f"Expected 3 rows, got {row_spans}"

    raw_cells = []
    heights = []
    for ry0, ry1 in row_spans:
        row_fg = fg[ry0:ry1, :]
        col_spans = find_spans(row_fg.sum(axis=0), min_gap=16)
        assert len(col_spans) == 4, f"Expected 4 cols in row {ry0}..{ry1}, got {col_spans}"
        for cx0, cx1 in col_spans:
            pad_y0 = max(0, ry0 - 2)
            pad_y1 = min(rgb.shape[0], ry1 + 2)
            pad_x0 = max(0, cx0 - 2)
            pad_x1 = min(rgb.shape[1], cx1 + 2)
            rgba = remove_green_bg(rgb[pad_y0:pad_y1, pad_x0:pad_x1])
            ys, _ = np.where(rgba[:, :, 3] > 128)
            heights.append(int(ys.max() - ys.min() + 1))
            raw_cells.append(rgba)

    scale_ref_h = float(heights[0])
    cells = [normalize_cell(c, scale_ref_h) for c in raw_cells]

    frames = [
        ("idle_0", cells[0]),
        ("idle_1", shift_frame(cells[0], dy=1)),
        ("blink", cells[1]),
        ("walk_0", cells[2]),
        ("walk_1", cells[3]),
        ("walk_2", cells[4]),
        ("walk_3", shift_frame(cells[3], dy=-1)),
        ("sit_0", cells[5]),
        ("sit_1", cells[6]),
        ("vibe_0", cells[7]),
        ("vibe_1", shift_frame(cells[7].transpose(Image.Transpose.FLIP_LEFT_RIGHT), dy=-1)),
        ("code_0", cells[8]),
        ("code_1", shift_frame(cells[8], dy=-1)),
        ("sleep_0", cells[9]),
        ("sleep_1", shift_frame(cells[9], dy=1)),
        ("dragged", cells[10]),
        ("happy", cells[11]),
        ("peek", cells[0]),
    ]

    os.makedirs(os.path.dirname(out_png), exist_ok=True)
    sheet = Image.new("RGBA", (TARGET_W * len(frames), TARGET_H), (0, 0, 0, 0))
    for i, (_, frame_im) in enumerate(frames):
        sheet.paste(frame_im, (i * TARGET_W, 0))
    sheet.save(out_png)

    if meta_path:
        with open(meta_path, "w") as f:
            json.dump(
                {
                    "frame_width": TARGET_W,
                    "frame_height": TARGET_H,
                    "frames": {name: i for i, (name, _) in enumerate(frames)},
                },
                f,
                indent=2,
            )

    if preview_path:
        os.makedirs(os.path.dirname(preview_path), exist_ok=True)
        grid_prev = Image.new("RGBA", (TARGET_W * 4 * 2, TARGET_H * 3 * 2), (28, 26, 38, 255))
        for idx, c_im in enumerate(cells):
            col = idx % 4
            row = idx // 4
            big = c_im.resize((TARGET_W * 2, TARGET_H * 2), Image.Resampling.NEAREST)
            grid_prev.paste(big, (col * TARGET_W * 2, row * TARGET_H * 2), big)
        grid_prev.save(preview_path)

    print(f"Saved {out_png} ({sheet.size[0]}x{sheet.size[1]}, {len(frames)} frames)")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", default=DEFAULT_SRC)
    parser.add_argument("--output", default=os.path.join(OUT_DIR, "sprites.png"))
    parser.add_argument("--meta", default=os.path.join(OUT_DIR, "sprites.json"))
    parser.add_argument(
        "--preview",
        default="/home/faewon/.gemini/antigravity-cli/brain/4532fa2f-dc2e-4f0f-9409-ce14f8ea7fee/scratch/cleaned_preview.png",
    )
    args = parser.parse_args()
    process_sheet(args.input, args.output, args.meta, args.preview)


if __name__ == "__main__":
    main()
