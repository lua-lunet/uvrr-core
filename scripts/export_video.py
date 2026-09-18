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


def node_card(d, cx, cy, label, colour, dead=False, w=150, h=96):
    x0, y0 = cx - w // 2, cy - h // 2
    fill = (241, 245, 249) if dead else CARD
    line = (228, 233, 242) if dead else BORDER
    d.rectangle([x0, y0, x0 + w, y0 + h], fill=fill, outline=line, width=2)
    d.rectangle([x0, y0, x0 + w, y0 + 30], fill=GREY if dead else colour)
    lw = d.textlength(label, font=F_TINY)
    d.text((cx - lw / 2, y0 + 7), label, font=F_TINY, fill=(255, 255, 255))
    return x0, y0, w, h


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
    node_card(d, 640, 360, "YOUR DATA", GREY if dead else BLUE)
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
        d.text((560, 210), "X  POWER LOST", font=F_BEAT, fill=RED)
    return img


def sc_split_brain(beat):
    img, d = base_frame(2, "Two nodes: the split brain",
                        {0: "copy every write to both",
                         1: "the wire breaks — each side is certain",
                         2: "two answers to one question"}[beat])
    node_card(d, 330, 360, "NODE A", PURPLE)
    node_card(d, 950, 360, "NODE B", MAGENTA)
    if beat == 0:
        d.line([410, 360, 870, 360], fill=GREY, width=5)
        d.text((600, 322), "the mirror link", font=F_SMALL, fill=MUTED)
    else:
        d.text((600, 322), "LINK DOWN", font=F_BEAT, fill=RED)
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
                                   ("NODE 1", "NODE 2", "NODE 3")):
        dead = (beat == 2 and name == "NODE 3")
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
    node_card(d, TRI[0][0], TRI[0][1] + 30, "PRIMARY", BLUE)
    node_card(d, TRI[1][0] + 80, TRI[1][1], "BACKUP", PURPLE)
    node_card(d, TRI[2][0] - 80, TRI[2][1], "BACKUP", MAGENTA)
    crown_at = TRI[0] if beat < 2 else (TRI[1][0] + 80, TRI[1][1] - 70)
    d.text((crown_at[0], crown_at[1] - 78), "♛", font=F_TITLE, fill=INK if beat < 2 else BLUE_DEEP, anchor="mm")
    return img


def sc_normal_path(beat):
    img, d = base_frame(7, "Normal operation: two round trips, no disk",
                        {0: "request → the primary appends",
                         1: "prepare → majority acknowledges → commit",
                         2: "zero disk writes on the path"}[beat])
    d.rectangle([100, 300, 260, 360], fill=CARD, outline=BORDER, width=2)
    d.text((180, 330), "CLIENT", font=F_SMALL, fill=INK, anchor="mm")
    node_card(d, 500, 330, "PRIMARY", BLUE)
    node_card(d, 900, 240, "BACKUP", PURPLE, w=130, h=80)
    node_card(d, 900, 430, "BACKUP", MAGENTA, w=130, h=80)
    if beat >= 0:
        arrow(d, (268, 330), (418, 330), INK)
        d.text((340, 300), "request", font=F_TINY, fill=MUTED, anchor="mm")
    if beat >= 1:
        arrow(d, (585, 310), (828, 250), GOLD)
        arrow(d, (585, 350), (828, 420), GOLD)
        d.text((700, 260), "prepare", font=F_TINY, fill=(184, 134, 11), anchor="mm")
        arrow(d, (828, 275), (590, 320), GREEN)
        arrow(d, (828, 400), (590, 345), GREEN)
        d.text((700, 400), "ack · ack", font=F_TINY, fill=GREEN, anchor="mm")
        d.rectangle([420, 470, 580, 540], fill=CARD, outline=BORDER, width=2)
        for i in range(4):
            d.rectangle([432 + i * 36, 492, 462 + i * 36, 520],
                        fill=(187, 247, 208) if beat >= 1 else (238, 242, 251))
        d.text((500, 528), "THE LOG", font=F_TINY, fill=MUTED, anchor="mm")
    if beat == 2:
        d.ellipse([1100, 290, 1160, 320], outline=GREY, width=3)
        d.rectangle([1095, 260, 1165, 292], fill=CARD, outline=GREY, width=3)
        d.line([1080, 340, 1180, 240], fill=RED, width=5)
        d.line([1080, 240, 1180, 340], fill=RED, width=5)
        d.text((1130, 372), "no disk on the path", font=F_TINY, fill=RED, anchor="mm")
    return img


def sc_failover(beat):
    img, d = base_frame(8, "Failover: the view change",
                        {0: "the primary dies — suspicion accrues",
                         1: "the pair exchanges what it knows · elects",
                         2: "the log continues exactly — service resumes"}[beat])
    d.rectangle([520, 170, 760, 220], fill=CARD, outline=BORDER, width=2)
    d.text((640, 195), "view 7" if beat < 2 else "view 8", font=F_BEAT, fill=BLUE_DEEP, anchor="mm")
    node_card(d, TRI[0][0], TRI[0][1] + 30, "PRIMARY", GREY if beat > 0 else BLUE,
              dead=beat > 0)
    if beat > 0:
        d.text((TRI[0][0], TRI[0][1] + 30), "X", font=F_TITLE, fill=GREY, anchor="mm")
    b1 = (TRI[1][0] + 80, TRI[1][1])
    b2 = (TRI[2][0] - 80, TRI[2][1])
    node_card(d, *b1, "BACKUP", PURPLE)
    node_card(d, *b2, "BACKUP", MAGENTA)
    if beat == 1:
        arrow(d, (b1[0] + 40, b1[1] - 30), (b2[0] - 40, b2[1] - 30), BLUE_DEEP)
        d.text((640, 400), "view change: exchange · elect", font=F_SMALL, fill=BLUE_DEEP, anchor="mm")
    if beat == 2:
        d.text((b2[0], b2[1] - 78), "♛", font=F_TITLE, fill=BLUE_DEEP, anchor="mm")
        d.text((640, 560), "SERVICE RESUMES — the log continues exactly",
               font=F_BEAT, fill=GREEN, anchor="mm")
    return img


def sc_new_identity(beat):
    img, d = base_frame(9, "A new identity: safety by construction",
                        {0: "the crash is FINAL for that identity",
                         1: "a new identity joins — weight 0, cannot vote",
                         2: "caught up, then promoted by a committed vote"}[beat])
    node_card(d, 330, 330, "IDENTITY N2", GREY, dead=True)
    if beat >= 0:
        d.rectangle([250, 372, 410, 404], outline=RED, width=3)
        d.text((330, 388), "F I N A L", font=F_SMALL, fill=RED, anchor="mm")
    if beat >= 1:
        arrow(d, (430, 330), (830, 330), ORANGE)
        d.text((630, 300), "the committed history streams", font=F_TINY, fill=ORANGE, anchor="mm")
        node_card(d, 950, 330, "N2'", ORANGE)
        if beat == 1:
            d.text((950, 372), "weight 0 — cannot vote", font=F_TINY, fill=MUTED, anchor="mm")
        else:
            d.rectangle([875, 362, 1025, 392], fill=GREEN)
            d.text((950, 377), "weight 1 — by a committed vote", font=F_TINY, fill=(255, 255, 255), anchor="mm")
    d.rectangle([560, 170, 720, 220], fill=CARD, outline=BORDER, width=2)
    d.text((640, 195), "era 12" if beat < 2 else "era 13", font=F_BEAT, fill=BLUE_DEEP, anchor="mm")
    return img


SCENES = [sc_one_node, sc_split_brain, sc_majority, sc_arithmetic,
          sc_five_then_three, sc_views, sc_normal_path, sc_failover,
          sc_new_identity]


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
