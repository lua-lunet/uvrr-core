#!/usr/bin/env python3
"""
Export the consistency-ladder explainer video. Renders three animated beats
per clip with Pillow (matching the player's scene beats), crossfades between
clips, and composites with podcast.mp3 via ffmpeg. The interactive player
(docs/animation/index.html) is the primary artifact; this is the linear
derivative for places that need a single file.
"""

import os
import json
import subprocess
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
ANIM_DIR = ROOT / "docs/animation"
FRAMES_DIR = ANIM_DIR / "video_frames"
CUES_JSON = ANIM_DIR / "cues.json"
AUDIO_MP3 = ANIM_DIR / "podcast.mp3"
OUTPUT_MP4 = ANIM_DIR / "uvrr_educational_explainer.mp4"

W, H = 1280, 720
FPS = 30
XFADE_FRAMES = 12  # short transition between clips

# IBM colour-blind safe palette
BG = (246, 248, 251)
CARD = (255, 255, 255)
BORDER = (196, 206, 224)
INK = (15, 23, 42)
MUTED = (91, 107, 132)
BLUE = (100, 143, 255)
BLUE_DEEP = (59, 91, 219)
PURPLE = (120, 94, 240)
MAGENTA = (220, 38, 127)
ORANGE = (254, 97, 0)
GREEN = (22, 163, 74)
RED = (220, 38, 38)
GOLD = (255, 176, 0)
GREY = (148, 163, 184)

TRI = [(640, 250), (480, 480), (800, 480)]  # triangle node centres


def get_font(size, bold=False):
    candidates = [
        ("/System/Library/Fonts/Supplemental/Arial Bold.ttf") if bold
        else ("/System/Library/Fonts/Supplemental/Arial.ttf"),
        "/System/Library/Fonts/Helvetica.ttc",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf" if bold
        else "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf" if bold
        else "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ]
    for p in candidates:
        if p and os.path.exists(p):
            try:
                return ImageFont.truetype(p, size)
            except Exception:
                continue
    return ImageFont.load_default()


F_KICK = get_font(26, bold=True)
F_TITLE = get_font(40, bold=True)
F_BEAT = get_font(24, bold=True)
F_SMALL = get_font(19)
F_TINY = get_font(15)


def base_frame(clip_no, title, beat_caption):
    img = Image.new("RGB", (W, H), BG)
    d = ImageDraw.Draw(img)
    d.rectangle([40, 40, W - 40, 128], fill=CARD, outline=BORDER, width=2)
    d.text((64, 56), f"RUNG {clip_no} OF 9", font=F_KICK, fill=BLUE_DEEP)
    d.text((64, 92), title, font=F_TITLE, fill=INK)
    if beat_caption:
        d.rectangle([40, H - 96, W - 40, H - 44], fill=CARD, outline=BORDER, width=2)
        tw = d.textlength(beat_caption, font=F_BEAT)
        d.text(((W - tw) / 2, H - 86), beat_caption, font=F_BEAT, fill=INK)
    return img, d


def node_card(d, cx, cy, label, colour, dead=False, w=None, h=None, leader=False):
    """A circle node in the classic explainer style: chunky coloured ring,
    the name inside (white when leading), mono label handled by the caller."""
    r = 46
    lead = (not dead) and label.lower() in ("primary", "your data")
    d.ellipse([cx - r - 4, cy + r - 10, cx + r + 4, cy + r + 12], fill=(226, 232, 240))
    if dead:
        d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=(238, 242, 246),
                  outline=(203, 213, 225), width=4)
    elif lead:
        d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=colour,
                  outline=(15, 23, 42), width=5)
    else:
        d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=CARD,
                  outline=colour, width=5)
    lw = d.textlength(label, font=F_SMALL)
    d.text((cx - lw / 2, cy - 11), label, font=F_SMALL,
           fill=(255, 255, 255) if lead else MUTED)
    return cx - r, cy - r, 2 * r, 2 * r


def dot(d, cx, cy, r, colour, label=None):
    d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=colour)
    if label:
        lw = d.textlength(label, font=F_TINY)
        d.text((cx - lw / 2, cy - r - 22), label, font=F_TINY, fill=MUTED)


def arrow(d, p1, p2, colour, width=3):
    d.line([p1, p2], fill=colour, width=width)
    # simple arrowhead
    import math
    ang = math.atan2(p2[1] - p1[1], p2[0] - p1[0])
    L, spread = 14, 0.5
    a = (p2[0] - L * math.cos(ang - spread), p2[1] - L * math.sin(ang - spread))
    b = (p2[0] - L * math.cos(ang + spread), p2[1] - L * math.sin(ang + spread))
    d.polygon([p2, a, b], fill=colour)


# ----------------------------------------------------------------------------
# One drawing function per clip; beat in {0, 1, 2} (early / mid / late).
# ----------------------------------------------------------------------------

def sc_one_node(beat):
    img, d = base_frame(1, "One node: your data, one machine",
                        {0: "one machine · one copy",
                         1: "it works — until the day it doesn't",
                         2: "everything is gone in one moment"}[beat])
    dead = beat == 2
    node_card(d, 640, 360, "your data", GREY if dead else BLUE)
    for i in range(5):
        x = 760 + (i % 3) * 40
        y = 300 + (i // 3) * 32
        if beat == 2:
            x += (i - 2) * 70
            y -= 90 * (1 if i % 2 else 0.6)
        d.rectangle([x, y, x + 30, y + 22], fill=GREY if dead else BLUE)
    if beat < 2:
        arrow(d, (200, 300), (540, 340), MUTED)
        d.text((220, 262), "requests", font=F_SMALL, fill=MUTED)
    else:
        d.text((560, 210), "✕ power lost", font=F_BEAT, fill=RED)
    return img


def sc_split_brain(beat):
    img, d = base_frame(2, "Two nodes: the split brain",
                        {0: "copy every write to both",
                         1: "the wire breaks — each side is certain",
                         2: "two answers to one question"}[beat])
    node_card(d, 330, 360, "node a", PURPLE)
    node_card(d, 950, 360, "node b", MAGENTA)
    if beat == 0:
        d.line([410, 360, 870, 360], fill=GREY, width=5)
        d.text((600, 322), "the mirror link", font=F_SMALL, fill=MUTED)
    else:
        d.text((600, 322), "link down", font=F_BEAT, fill=RED)
        d.line([570, 344, 630, 380], fill=RED, width=4)
        d.line([630, 344, 570, 380], fill=RED, width=4)
    va, vb = ("x=1", "x=2") if beat == 2 else ("x=1", "x=1")
    d.text((330, 350), va, font=F_TITLE, fill=PURPLE, anchor="mm")
    d.text((950, 350), vb, font=F_TITLE, fill=MAGENTA, anchor="mm")
    if beat == 2:
        d.text((640, 560), "two histories of one truth", font=F_BEAT, fill=RED, anchor="mm")
    return img


def sc_majority(beat):
    img, d = base_frame(3, "Three nodes: the majority",
                        {0: "any two of the three overlap",
                         1: "cloud regions: separate power and networks",
                         2: "lose one — the memory survives in the pair"}[beat])
    for (cx, cy), col, name in zip(TRI, (BLUE, PURPLE, MAGENTA),
                                   ("node 1", "node 2", "node 3")):
        dead = (beat == 2 and name == "node 3")
        node_card(d, cx, cy, name, col, dead=dead)
        if not dead and beat >= 0:
            dot(d, cx, cy + 34, 11, (187, 247, 208) if beat >= 0 else CARD)
    if beat == 2:
        d.text((800, 480), "lost", font=F_SMALL, fill=GREY, anchor="mm")
    if beat >= 1:
        for a, b in (((180, 180), (1100, 180)),):
            d.line([a, b], fill=(143, 176, 255), width=3)
        for i, x in enumerate((180, 640, 1100)):
            dot(d, x, 180, 9, BLUE_DEEP)
            d.text((x, 148), f"region {i+1}", font=F_TINY, fill=MUTED, anchor="mm")
        d.text((640, 212), "fast interconnects", font=F_TINY, fill=BLUE_DEEP, anchor="mm")
    return img


def sc_arithmetic(beat):
    img, d = base_frame(4, "The arithmetic: 2F+1",
                        {0: "tolerate F failures — run 2F+1",
                         1: "five nodes: lose any two",
                         2: "below a majority: stop and wait"}[beat])
    d.rectangle([490, 170, 790, 240], fill=CARD, outline=BORDER, width=2)
    d.text((640, 205), "2F + 1", font=F_TITLE, fill=INK, anchor="mm")
    for row, (y, col, n, lbl) in enumerate([(330, BLUE, 3, "F = 1  →  lose 1 of 3"),
                                             (450, PURPLE, 5, "F = 2  →  lose 2 of 5")]):
        alive = n if beat < 2 else (n - (0 if row == 0 else 2))
        for i in range(n):
            r = 17
            cx = 340 + i * 56
            dead = beat == 2 and row == 1 and i >= 3
            dot(d, cx, y, r, GREY if dead else col)
        lw = d.textlength(lbl, font=F_SMALL)
        d.text((700 + (620 - 700 - lw) / 2 + 40, y - 10), lbl, font=F_SMALL, fill=GREEN if beat < 2 else INK)
    if beat == 2:
        d.text((640, 560), "refusing to answer is safe — a stale answer is not",
              font=F_BEAT, fill=RED, anchor="mm")
    return img


def sc_five_then_three(beat):
    img, d = base_frame(5, "Five, then back to three",
                        {0: "five nodes — the service never dips",
                         1: "two lost — the majority survives",
                         2: "back to three for the rest of the walk"}[beat])
    d.rectangle([390, 150, 890, 182], fill=(238, 242, 251), outline=BORDER)
    d.rectangle([390, 150, 890, 182], fill=(187, 247, 208))
    d.text((640, 166), "SERVICE — never interrupted", font=F_TINY, fill=(22, 101, 52), anchor="mm")
    cols = [BLUE, PURPLE, BLUE_DEEP, MAGENTA, ORANGE]
    if beat < 2:
        for i, c in enumerate(cols):
            dead = beat == 1 and i in (1, 3)
            dot(d, 400 + i * 120, 340, 20, GREY if dead else c)
    else:
        for (cx, cy), c in zip(TRI, (BLUE, PURPLE, MAGENTA)):
            dot(d, cx, cy, 20, c)
        d.text((640, 560), "same arithmetic at any size", font=F_SMALL, fill=MUTED, anchor="mm")
    if beat == 1:
        d.text((640, 420), "two lost — the majority survives", font=F_BEAT, fill=RED, anchor="mm")
    return img


def sc_views(beat):
    img, d = base_frame(6, "Views and the primary",
                        {0: "one view — one primary",
                         1: "the primary decides the order",
                         2: "the view changes — leadership moves"}[beat])
    d.rectangle([520, 170, 760, 220], fill=CARD, outline=BORDER, width=2)
    d.text((640, 195), "view 7" if beat < 2 else "view 8", font=F_BEAT, fill=BLUE_DEEP, anchor="mm")
    node_card(d, TRI[0][0], TRI[0][1] + 30, "primary", BLUE)
    node_card(d, TRI[1][0] + 80, TRI[1][1], "backup", PURPLE)
    node_card(d, TRI[2][0] - 80, TRI[2][1], "backup", MAGENTA)
    crown_at = TRI[0] if beat < 2 else (TRI[1][0] + 80, TRI[1][1] - 70)
    d.text((crown_at[0], crown_at[1] - 78), "♛", font=F_TITLE, fill=INK if beat < 2 else BLUE_DEEP, anchor="mm")
    return img


def sc_normal_path(beat):
    img, d = base_frame(7, "Normal operation: two round trips, no disk",
                        {0: "the leader acknowledges its own message",
                         1: "one reply from either backup is enough — a majority has spoken",
                         2: "committed — no disk on the path"}[beat])
    # two data centres, visibly separated; the far wire is twice as long
    d.rounded_rectangle([250, 120, 830, 560], radius=24,
                        fill=(244, 247, 253), outline=BORDER, width=2)
    d.text((280, 140), "region 1", font=F_TINY, fill=GREY)
    d.rounded_rectangle([950, 170, 1240, 460], radius=24,
                        fill=(244, 247, 253), outline=BORDER, width=2)
    d.text((980, 190), "region 2", font=F_TINY, fill=GREY)
    d.rectangle([90, 320, 210, 380], fill=CARD, outline=BORDER, width=2)
    d.text((150, 350), "client", font=F_SMALL, fill=INK, anchor="mm")
    # base wires: client, short hop, long haul
    d.line([218, 340, 352, 296], fill=BORDER, width=3)
    d.line([452, 268, 648, 268], fill=BORDER, width=3)
    d.line([452, 304, 1043, 316], fill=BORDER, width=3)
    node_card(d, 400, 280, "primary", BLUE)
    node_card(d, 700, 280, "backup", PURPLE)
    node_card(d, 1095, 310, "backup", MAGENTA)
    d.text((436, 244), "✓", font=F_BEAT, fill=GREEN, anchor="mm")
    slot_fill = {0: (255, 226, 168) if beat < 2 else (187, 247, 208),
                 1: (238, 242, 251), 2: (238, 242, 251), 3: (238, 242, 251)}
    d.rectangle([340, 370, 460, 430], fill=CARD, outline=BORDER, width=2)
    d.text((400, 384), "the log", font=F_TINY, fill=MUTED, anchor="mm")
    for i in range(4):
        d.rectangle([354 + i * 24, 396, 372 + i * 24, 414],
                    fill=slot_fill[i], outline=BORDER, width=1)
    if beat == 0:
        arrow(d, (218, 345), (350, 296), INK)
        d.text((300, 330), "request", font=F_TINY, fill=MUTED, anchor="mm")
        d.text((400, 470), "the leader acknowledges its own message",
               font=F_TINY, fill=(22, 101, 52), anchor="mm")
    if beat == 1:
        arrow(d, (452, 262), (648, 262), GOLD)
        arrow(d, (452, 296), (1043, 310), GOLD)
        d.text((545, 240), "prepare", font=F_TINY, fill=(184, 134, 11), anchor="mm")
        d.text((620, 356), "the same prepare, two distances",
               font=F_TINY, fill=MUTED, anchor="mm")
        arrow(d, (648, 306), (456, 316), GREEN)
        d.text((550, 336), "ack", font=F_TINY, fill=GREEN, anchor="mm")
        arrow(d, (1043, 340), (790, 346), GREEN)
        dot(d, 760, 348, 8, GREEN)
        d.rounded_rectangle([300, 150, 830, 200], radius=18,
                            fill=CARD, outline=GREEN, width=2)
        d.text((565, 175), "quorum: primary + one reply = two of three",
               font=F_TINY, fill=(22, 101, 52), anchor="mm")
        arrow(d, (348, 262), (216, 326), GREEN)
        d.text((296, 272), "response", font=F_TINY, fill=GREEN, anchor="mm")
    if beat == 2:
        d.text((738, 244), "✓", font=F_BEAT, fill=GREEN, anchor="mm")
        d.text((1134, 276), "✓", font=F_BEAT, fill=GREEN, anchor="mm")
        d.text((1095, 380), "also counted", font=F_TINY, fill=GREY, anchor="mm")
        d.ellipse([1064, 526, 1116, 544], outline=GREY, width=3)
        d.rectangle([1060, 498, 1120, 526], fill=CARD, outline=GREY, width=3)
        d.line([1050, 560, 1130, 466], fill=RED, width=5)
        d.line([1050, 466, 1130, 560], fill=RED, width=5)
        d.text((1090, 590), "no disk on the path", font=F_TINY, fill=RED, anchor="mm")
    return img


def sc_failover(beat):
    img, d = base_frame(8, "Failover: the view change",
                        {0: "the heartbeats stop — the survivors call a view change",
                         1: "a majority elects — the new view is installed",
                         2: "service resumes — the log continues exactly"}[beat])
    d.rectangle([520, 170, 760, 220], fill=CARD, outline=BORDER, width=2)
    d.text((640, 195), "view 7" if beat < 1 else "view 8", font=F_BEAT,
           fill=BLUE_DEEP, anchor="mm")
    p = (TRI[0][0], TRI[0][1] + 30)          # the crashed primary, top
    b1 = (TRI[1][0] + 80, TRI[1][1])         # left survivor -> new primary
    b2 = (TRI[2][0] - 80, TRI[2][1])         # right survivor
    # wires: to the dead primary and between the survivors
    d.line([(623, 323), (577, 437)], fill=BORDER, width=3)
    d.line([(657, 323), (703, 437)], fill=BORDER, width=3)
    d.line([(606, 480), (674, 480)], fill=BORDER, width=3)
    node_card(d, *p, "primary", GREY, dead=True)
    d.text(p, "✕", font=F_TITLE, fill=GREY, anchor="mm")
    # the dead sockets at the wire ends go dark
    dot(d, 623, 323, 4, GREY)
    dot(d, 657, 323, 4, GREY)
    node_card(d, *b1, "backup" if beat < 1 else "primary", PURPLE)
    node_card(d, *b2, "backup", MAGENTA)
    if beat >= 1:
        d.text((b1[0], b1[1] - 78), "♛", font=F_TITLE, fill=BLUE_DEEP, anchor="mm")
    if beat == 0:
        # start-view-change: each survivor sends to every other node; the
        # dots aimed at the crashed node die at its dark socket
        arrow(d, (606, 472), (674, 472), BLUE_DEEP)
        dot(d, 628, 355, 8, BLUE_DEEP)
        dot(d, 652, 355, 8, BLUE_DEEP)
        d.text((640, 548), "start view change · v 8", font=F_TINY,
               fill=BLUE_DEEP, anchor="mm")
        # do-view-change: the log travels to the new primary
        d.rounded_rectangle([604, 436, 676, 468], radius=6, fill=CARD,
                            outline=BORDER, width=2)
        for i in range(4):
            d.rectangle([610 + i * 15, 444, 622 + i * 15, 460],
                        fill=(187, 247, 208), outline=(134, 211, 164), width=1)
        d.text((640, 572), "do view change · log + last committed", font=F_TINY,
               fill=MUTED, anchor="mm")
    if beat == 1:
        # a majority (itself + one) elects; the new view goes to every node,
        # retained at the dead node's socket
        d.ellipse([b1[0] - 64, b1[1] - 64, b1[0] + 64, b1[1] + 64],
                  outline=GREEN, width=3)
        d.text((410, 430), "quorum: two of three", font=F_TINY, fill=GREEN,
               anchor="mm")
        arrow(d, (b1[0] + 34, b1[1] - 34), (b2[0] - 44, b2[1] - 6), GREEN)
        arrow(d, (b1[0] + 18, b1[1] - 48), (628, 332), GREEN)
        dot(d, 631, 329, 7, GREEN)
        d.text((640, 548), "new view · v 8 · log", font=F_TINY, fill=GREEN,
               anchor="mm")
    if beat == 2:
        # the log continues exactly, and heartbeats resume from the new primary
        d.rectangle([170, 300, 320, 400], fill=CARD, outline=BORDER, width=2)
        d.text((245, 322), "the log", font=F_TINY, fill=MUTED, anchor="mm")
        for i in range(4):
            d.rectangle([186 + i * 30, 340, 212 + i * 30, 362],
                        fill=(187, 247, 208), outline=(134, 211, 164), width=1)
        d.text((245, 384), "committed prefix", font=F_TINY, fill=GREY, anchor="mm")
        arrow(d, (606, 474), (674, 474), GREEN)
        arrow(d, (674, 486), (606, 486), GREEN)
        d.text((640, 548), "heartbeats resume", font=F_TINY, fill=GREEN, anchor="mm")
    return img


SCENES = [sc_one_node, sc_split_brain, sc_majority, sc_arithmetic,
          sc_five_then_three, sc_views, sc_normal_path, sc_failover]


def main():
    cues = json.loads(CUES_JSON.read_text())
    clips = cues["clips"]
    if len(clips) != len(SCENES):
        raise SystemExit(f"cues.json has {len(clips)} clips; expected {len(SCENES)}")

    FRAMES_DIR.mkdir(parents=True, exist_ok=True)
    for old in FRAMES_DIR.glob("f_*.png"):
        old.unlink()

    frame_paths = []
    for i, clip in enumerate(clips):
        dur = clip["end_time"] - clip["start_time"]
        beat_dur = dur / 3.0
        closing = None
        for beat in range(3):
            beats = [SCENES[i](beat)]
            n = max(1, int(round(beat_dur * FPS)))
            # hold each beat's frame; a gentle zoom keeps it alive
            for k in range(n):
                zoom = 1.0 + 0.012 * (k / n)
                zw, zh = int(W / zoom), int(H / zoom)
                ox, oy = (W - zw) // 2, (H - zh) // 2
                frame = beats[0].resize((zw, zh), Image.LANCZOS)
                out = Image.new("RGB", (W, H), BG)
                out.paste(frame, (ox, oy))
                p = FRAMES_DIR / f"f_{len(frame_paths):05d}.png"
                out.save(p)
                frame_paths.append(p)
                closing = out
        # the inter-clip gap is part of the timeline: hold the closing frame
        gap = 0.0
        if i + 1 < len(clips):
            gap = clips[i + 1]["start_time"] - clip["end_time"]
        for _ in range(int(round(gap * FPS))):
            p = FRAMES_DIR / f"f_{len(frame_paths):05d}.png"
            closing.save(p)
            frame_paths.append(p)
        print(f"clip {i+1}/9 rendered: {clip['title']}")

    concat = FRAMES_DIR / "concat_frames.txt"
    with open(concat, "w") as f:
        for p in frame_paths:
            f.write(f"file '{p.resolve()}'\n")

    print(f"Compositing {len(frame_paths)} frames with narration -> {OUTPUT_MP4}")
    subprocess.run([
        "ffmpeg", "-y", "-r", str(FPS), "-f", "concat", "-safe", "0",
        "-i", str(concat), "-i", str(AUDIO_MP3),
        "-c:v", "libx264", "-pix_fmt", "yuv420p", "-preset", "medium",
        "-c:a", "aac", "-b:a", "160k",
        str(OUTPUT_MP4)
    ], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    probe = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration",
         "-of", "default=noprint_wrappers=1:nokey=1", str(OUTPUT_MP4)],
        stdout=subprocess.PIPE, text=True, check=True)
    print(f"Video: {OUTPUT_MP4} ({float(probe.stdout.strip()):.1f}s)")


if __name__ == "__main__":
    main()
