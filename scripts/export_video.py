#!/usr/bin/env python3
"""
Export high-definition MP4 educational video for uVRR Explainer.
Generates 1920x1080 keyframes with Pillow and composites with podcast.mp3 via ffmpeg.
"""

import os
import sys
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

# 1920x1080 Resolution
W, H = 1920, 1080

# IBM Colour-Blind Safe Palette
BG_COLOR = (248, 250, 252)        # #F8FAFC
CARD_BG = (255, 255, 255)         # #FFFFFF
BORDER_COLOR = (226, 232, 240)    # #E2E8F0
TEXT_DARK = (15, 23, 42)          # #0F172A
TEXT_MUTED = (100, 116, 139)      # #64748B
N0_BLUE = (100, 143, 255)         # #648FFF
N1_PURPLE = (120, 94, 240)        # #785EF0
N2_MAGENTA = (220, 38, 127)       # #DC267F
N2P_ORANGE = (254, 97, 0)         # #FE6100
MSG_GOLD = (255, 176, 0)          # #FFB000
GREEN = (22, 163, 74)             # #16A34A
RED = (220, 38, 38)               # #DC2626
NAVY = (15, 23, 42)               # #0F172A

def get_font(size, bold=False):
    # Try system fonts on macOS / Linux
    font_paths = [
        "/System/Library/Fonts/Helvetica.ttc",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/System/Library/Fonts/SFNS.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf" if bold else "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf" if bold else "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf"
    ]
    for p in font_paths:
        if os.path.exists(p):
            try:
                return ImageFont.truetype(p, size)
            except Exception:
                continue
    return ImageFont.load_default()

def draw_superblock_gauges(draw, cx, cy, count_flushed, count_stopped, count_running):
    total = 4
    start_x = cx - 30
    for i in range(total):
        x = start_x + i * 20
        fill_c = (226, 232, 240) # unvouched
        if i < count_flushed:
            fill_c = GREEN
        elif i < count_flushed + count_stopped:
            fill_c = N0_BLUE
        elif i < count_flushed + count_stopped + count_running:
            fill_c = (217, 119, 6) # amber
            
        draw.ellipse([x - 7, cy - 7, x + 7, cy + 7], fill=fill_c, outline=NAVY, width=2)

def draw_curved_arrow(draw, start, end, label, font):
    x1, y1 = start
    x2, y2 = end
    # Midpoint
    mx = (x1 + x2) // 2
    my = (y1 + y2) // 2 - 40
    
    # Draw arc segments
    points = []
    for step in range(21):
        t = step / 20.0
        # Quadratic bezier
        px = (1 - t)**2 * x1 + 2 * (1 - t) * t * mx + t**2 * x2
        py = (1 - t)**2 * y1 + 2 * (1 - t) * t * my + t**2 * y2
        points.append((px, py))
        
    for i in range(len(points) - 1):
        draw.line([points[i], points[i+1]], fill=MSG_GOLD, width=4)
        
    # Draw arrowhead
    draw.polygon([(x2, y2), (x2 - 12, y2 - 8), (x2 - 8, y2 + 12)], fill=MSG_GOLD)
    
    # Label badge
    draw.text((mx - len(label)*4, my - 20), label, fill=(180, 83, 9), font=font)

def render_frame(stage_data, output_png):
    img = Image.new("RGB", (W, H), BG_COLOR)
    draw = ImageDraw.Draw(img)

    f_kicker = get_font(18, bold=True)
    f_title = get_font(34, bold=True)
    f_sub = get_font(20, bold=False)
    f_node_title = get_font(22, bold=True)
    f_node_sub = get_font(16, bold=False)
    f_msg = get_font(17, bold=True)
    f_rule_title = get_font(22, bold=True)
    f_rule_body = get_font(19, bold=False)
    f_subtitles = get_font(20, bold=False)

    # 1. Header Card
    draw.rounded_rectangle([60, 40, W - 60, 160], radius=16, fill=CARD_BG, outline=BORDER_COLOR, width=2)
    draw.text((90, 56), "UVRR DISTRIBUTED SYSTEMS EXPLAINER", fill=N0_BLUE, font=f_kicker)
    draw.text((90, 84), stage_data["title"], fill=TEXT_DARK, font=f_title)
    draw.text((90, 126), f"Lean 4.33.1 Verified Foundation  |  TigerBeetle Superblock Architecture  |  Stage {stage_data['num']} of 8", fill=TEXT_MUTED, font=f_sub)

    # Stage Badge Pill
    draw.rounded_rectangle([W - 260, 75, W - 90, 125], radius=25, fill=(224, 231, 255), outline=N0_BLUE, width=2)
    draw.text((W - 238, 88), f"STAGE {stage_data['num']} / 8", fill=(55, 48, 163), font=f_kicker)

    # 2. Main Cluster Viewport Card
    draw.rounded_rectangle([60, 180, W - 60, 720], radius=16, fill=CARD_BG, outline=BORDER_COLOR, width=2)

    # Static Network Topology Lines
    draw.line([(960, 310), (520, 570)], fill=BORDER_COLOR, width=3)
    draw.line([(960, 310), (1400, 570)], fill=BORDER_COLOR, width=3)
    draw.line([(520, 570), (1400, 570)], fill=BORDER_COLOR, width=3)

    # Center Status Pill
    draw.rounded_rectangle([960 - 320, 450, 960 + 320, 510], radius=30, fill=CARD_BG, outline=(203, 213, 225), width=2)
    draw.text((960 - len(stage_data['pill'])*5, 467), stage_data["pill"], fill=TEXT_DARK, font=f_node_title)

    # Draw Nodes
    # Node 0: Top Center (960, 310)
    draw.ellipse([960 - 75, 310 - 75, 960 + 75, 310 + 75], fill=N0_BLUE, outline=NAVY, width=4)
    draw.text((960 - 55, 292), "N₀ (Leader)", fill=(255, 255, 255), font=f_node_title)
    draw.text((960 - 50, 324), stage_data["n0_role"], fill=(255, 255, 255), font=f_node_sub)
    draw_superblock_gauges(draw, 960, 360, 4, 0, 0)
    draw.text((960 - 32, 400), "Weight: 1", fill=NAVY, font=f_node_sub)

    # Node 1: Bottom Left (520, 570)
    draw.ellipse([520 - 75, 570 - 75, 520 + 75, 570 + 75], fill=N1_PURPLE, outline=NAVY, width=4)
    draw.text((520 - 55, 552), "N₁ (Backup)", fill=(255, 255, 255), font=f_node_title)
    draw.text((520 - 45, 584), stage_data["n1_role"], fill=(255, 255, 255), font=f_node_sub)
    draw_superblock_gauges(draw, 520, 620, 4, 0, 0)
    draw.text((520 - 32, 660), "Weight: 1", fill=NAVY, font=f_node_sub)

    # Node 2: Bottom Right (1400, 570)
    n2_fill = stage_data["n2_fill"]
    draw.ellipse([1400 - 75, 570 - 75, 1400 + 75, 570 + 75], fill=n2_fill, outline=NAVY, width=4)
    draw.text((1400 - 65, 552), stage_data["n2_title"], fill=(255, 255, 255), font=f_node_title)
    draw.text((1400 - 60, 584), stage_data["n2_role"], fill=(255, 255, 255), font=f_node_sub)
    
    # Superblocks on N2
    sb2 = stage_data["n2_sb"]
    draw_superblock_gauges(draw, 1400, 620, sb2[0], sb2[1], sb2[2])
    draw.text((1400 - 32, 660), f"Weight: {stage_data['n2_weight']}", fill=NAVY, font=f_node_sub)

    # Crash mark if crashed
    if stage_data.get("crashed"):
        draw.line([(1400 - 50, 570 - 50), (1400 + 50, 570 + 50)], fill=RED, width=12)
        draw.line([(1400 + 50, 570 - 50), (1400 - 50, 570 + 50)], fill=RED, width=12)

    # Message flight
    for msg in stage_data.get("messages", []):
        draw_curved_arrow(draw, msg["start"], msg["end"], msg["label"], f_msg)

    # 3. Lower Details Card (740 to 1040)
    draw.rounded_rectangle([60, 740, W - 60, 1030], radius=16, fill=CARD_BG, outline=BORDER_COLOR, width=2)

    # Rule Box
    r_color = stage_data.get("rule_color", N0_BLUE)
    draw.rounded_rectangle([90, 760, W - 90, 830], radius=10, fill=(248, 250, 252), outline=r_color, width=2)
    draw.text((110, 775), "PROTOCOL INVARIANT: ", fill=r_color, font=f_rule_title)
    draw.text((360, 778), stage_data["rule_text"], fill=TEXT_DARK, font=f_rule_body)

    # Bullet points
    for idx, b in enumerate(stage_data["bullets"]):
        draw.text((110, 845 + idx * 30), f"•  {b}", fill=TEXT_DARK, font=f_rule_body)

    # Dialogue subtitle banner at the bottom
    draw.rounded_rectangle([90, 940, W - 90, 1010], radius=10, fill=(239, 246, 255), outline=N0_BLUE, width=1)
    draw.text((110, 952), stage_data["speaker"], fill=N0_BLUE, font=f_kicker)
    draw.text((110, 975), stage_data["quote"], fill=TEXT_DARK, font=f_subtitles)

    img.save(output_png)
    print(f"Rendered frame: {output_png}")

STAGES_METADATA = [
    {
        "num": 1,
        "title": "Stage 1: Steady-State Replication at Genesis Era 1",
        "pill": "Era 1: [1, 1, 1] Volatile Normal Path",
        "n0_role": "Era 1 | Primary",
        "n1_role": "Era 1 | Normal",
        "n2_title": "N₂ (Backup)",
        "n2_role": "Era 1 | Normal",
        "n2_fill": N2_MAGENTA,
        "n2_weight": 1,
        "n2_sb": (4, 0, 0),
        "crashed": False,
        "messages": [
            {"start": (960, 310), "end": (520, 570), "label": "Prepare #101"},
            {"start": (960, 310), "end": (1400, 570), "label": "Prepare #101"}
        ],
        "rule_color": GREEN,
        "rule_text": "Zero disk writes on normal path. Consensus held in volatile quorum memory.",
        "bullets": [
            "Genesis Era 1 initialized with unit weights [1, 1, 1]. All replicas in vouched clean state.",
            "Node 0 elected primary; Nodes 1 & 2 process Prepares and Commits in volatile memory.",
            "Any 2-of-3 strict weighted majority commits client operations with immediate linearizability."
        ],
        "speaker": "ALEX (HOST) & DR. MORGAN (RESEARCHER)",
        "quote": "\"Like classic VRR-2012, normal operations require zero disk writes on the critical path!\""
    },
    {
        "num": 2,
        "title": "Stage 2: Node 2 Crashes (Power Loss / Sudden SIGKILL)",
        "pill": "N₂ Crashed: Volatile State Wiped — Zero Disk Writes",
        "n0_role": "Era 1 | Primary",
        "n1_role": "Era 1 | Normal",
        "n2_title": "N₂ (Crashed)",
        "n2_role": "Dead / Unresponsive",
        "n2_fill": (148, 163, 184),
        "n2_weight": 1,
        "n2_sb": (0, 0, 3),
        "crashed": True,
        "messages": [
            {"start": (960, 310), "end": (520, 570), "label": "Prepare #102"},
            {"start": (520, 570), "end": (960, 310), "label": "PrepareOk #102"}
        ],
        "rule_color": RED,
        "rule_text": "A crash is final for the protocol identity! Old identity is permanently retired.",
        "bullets": [
            "Node 2 loses power or process dies. In-memory promises and uncommitted state are lost.",
            "Zero disk writes occur during failure (diskless normal path preserves maximum throughput).",
            "Surviving quorum {N₀, N₁} retains mass 2 of 3 and continues committing client requests."
        ],
        "speaker": "DR. MORGAN (CONSENSUS RESEARCHER)",
        "quote": "\"uVRR eliminates the Michael et al. amnesia bug with one rule: a crash is final for identity!\""
    },
    {
        "num": 3,
        "title": "Stage 3: Superblock Quorum Read at the Boot Gate",
        "pill": "Superblock Quorum Read: [RUNNING, STOPPED] ≡ Crashed",
        "n0_role": "Era 1 | Primary",
        "n1_role": "Era 1 | Normal",
        "n2_title": "N₂ (Booting)",
        "n2_role": "Superblock Read",
        "n2_fill": (203, 213, 225),
        "n2_weight": 0,
        "n2_sb": (0, 1, 3),
        "crashed": False,
        "messages": [
            {"start": (1400, 670), "end": (1400, 570), "label": "4-Copy Superblock Read"}
        ],
        "rule_color": (217, 119, 6),
        "rule_text": "Three marker states (RUNNING, STOPPED, FLUSHED) disambiguate torn 4-copy writes.",
        "bullets": [
            "Node powers on and reads 4 spaced TigerBeetle-style superblock copies from disk.",
            "Absence of FLUSHED quorum proves dirty crash. Minimum progress classifies start as Crashed.",
            "Zero disk flush is paid at startup boundary! Uninitialised node prioritises fast recovery."
        ],
        "speaker": "DR. MORGAN (CONSENSUS RESEARCHER)",
        "quote": "\"Because disk writes tear, you need 3 states. No FLUSHED quorum proves the node died dirty.\""
    },
    {
        "num": 4,
        "title": "Stage 4: Blank Boot at Era 0 & Reincarnation as N₂'",
        "pill": "Reincarnation: N₂' Born at Era 0 (Standby / Amnesia-Free)",
        "n0_role": "Era 1 | Primary",
        "n1_role": "Era 1 | Normal",
        "n2_title": "N₂' (Newborn)",
        "n2_role": "Era 0 | Standby",
        "n2_fill": N2P_ORANGE,
        "n2_weight": 0,
        "n2_sb": (0, 0, 0),
        "crashed": False,
        "messages": [],
        "rule_color": (217, 119, 6),
        "rule_text": "Resuming a crashed identity is unrepresentable. N₂' boots blank with zero promises.",
        "bullets": [
            "Old identity N₂ is dead. Replacement incarnates as N₂' (Orange #FE6100) at Era 0.",
            "Genesis era is 1: in-cluster state sync to an Era 0 node is refused by construction.",
            "A newborn has made no prior commitments, guaranteeing linearizability across amnesia."
        ],
        "speaker": "DR. MORGAN (CONSENSUS RESEARCHER)",
        "quote": "\"A newborn has made no prior promises, so amnesia is harmless by construction!\""
    },
    {
        "num": 5,
        "title": "Stage 5: Outer Join Gossip Broadcast (Frontier 0)",
        "pill": "Outer Gossip: N₂' Broadcasts Join Request to All Known Nodes",
        "n0_role": "Era 1 | Primary",
        "n1_role": "Era 1 | Normal",
        "n2_title": "N₂' (Joiner)",
        "n2_role": "Era 0 | Gossiping",
        "n2_fill": N2P_ORANGE,
        "n2_weight": 0,
        "n2_sb": (0, 0, 0),
        "crashed": False,
        "messages": [
            {"start": (1400, 570), "end": (960, 310), "label": "JoinGossip (frontier=0)"},
            {"start": (1400, 570), "end": (520, 570), "label": "JoinGossip (frontier=0)"}
        ],
        "rule_color": N0_BLUE,
        "rule_text": "Gossip discovery runs outside consensus engine without corrupting active view fences.",
        "bullets": [
            "Node 2 Prime broadcasts authenticated JoinGossip envelopes stating its frontier (slot 0).",
            "Any peer hearing gossip responds with the latest known cluster configuration and era.",
            "Node 2 Prime adopts the maximum reported era without assuming it has seen the latest."
        ],
        "speaker": "ALEX (HOST) & DR. MORGAN",
        "quote": "\"Through an outer gossip protocol that sits completely outside the consensus engine!\""
    },
    {
        "num": 6,
        "title": "Stage 6: Leader Witness Stream & Implicit Promise Telescoping",
        "pill": "Witness Stream: Leader Streams Missed Suffix + Commits",
        "n0_role": "Era 1 | Primary",
        "n1_role": "Era 1 | Normal",
        "n2_title": "N₂' (Witness)",
        "n2_role": "Era 1 | Catch-up",
        "n2_fill": N2P_ORANGE,
        "n2_weight": 0,
        "n2_sb": (0, 0, 0),
        "crashed": False,
        "messages": [
            {"start": (960, 310), "end": (1400, 570), "label": "Stream(Phase2s 1..100, Commit)"}
        ],
        "rule_color": N0_BLUE,
        "rule_text": "Equivalence Theorem: Gossip ≡ Recovery minus memory. Stream telescopes in 1 RTT.",
        "bullets": [
            "Leader N₀ registers N₂' on its gossip-witness list and pushes missing log suffix.",
            "Accepting Phase 2 at view v raises node's promise floor to v (implicit promise telescoping).",
            "Lean theorems H1–H5 prove the witness reconstructs the leader's exact committed history."
        ],
        "speaker": "DR. MORGAN (CONSENSUS RESEARCHER)",
        "quote": "\"Gossip equals recovery, with the memory precondition removed! Telescopes in 1 RTT.\""
    },
    {
        "num": 7,
        "title": "Stage 7: Two-Step Reconfiguration via Leader Casting Vote",
        "pill": "Reconfiguration: Era 2 [1, 1, 0, 0] → Era 3 [1, 1, 1] Seated",
        "n0_role": "Era 3 | Primary",
        "n1_role": "Era 3 | Normal",
        "n2_title": "N₂' (Promoted)",
        "n2_role": "Era 3 | Seated",
        "n2_fill": N2P_ORANGE,
        "n2_weight": 1,
        "n2_sb": (0, 0, 0),
        "crashed": False,
        "messages": [
            {"start": (960, 310), "end": (520, 570), "label": "Reconfig Batch E2/E3"},
            {"start": (960, 310), "end": (1400, 570), "label": "Reconfig Batch E2/E3"}
        ],
        "rule_color": N0_BLUE,
        "rule_text": "Turner Leader Overlap: 2 committed batches safely transfer voting power without stalls.",
        "bullets": [
            "Era 2: Dead N₂ decremented 1→0; N₂' joins as weight-0 standby (weights [1, 1, 0, 0]).",
            "Era 3: N₂' incremented 0→1; old N₂ evicted (weights [1, 1, 1]).",
            "Turner Lemma 2 proves consecutive strict majorities intersect via leader casting vote."
        ],
        "speaker": "DR. MORGAN (CONSENSUS RESEARCHER)",
        "quote": "\"Turner leader overlap in action: every consecutive era maintains overlapping majorities!\""
    },
    {
        "num": 8,
        "title": "Stage 8: Seated Observation & Deferred RUNNING Latch",
        "pill": "Cluster Restored: Engine Mints Rejoined → 4× RUNNING Latched",
        "n0_role": "Era 3 | Primary",
        "n1_role": "Era 3 | Normal",
        "n2_title": "N₂' (Active)",
        "n2_role": "Era 3 | LATCHED",
        "n2_fill": N2P_ORANGE,
        "n2_weight": 1,
        "n2_sb": (0, 0, 4),
        "crashed": False,
        "messages": [
            {"start": (1400, 570), "end": (1400, 670), "label": "4× RUNNING Latch Forced"}
        ],
        "rule_color": GREEN,
        "rule_text": "Deferred Latch Safety: Flush kept off critical path until node was seated as normal voter.",
        "bullets": [
            "Replica::rejoined() observes Normal status at voting weight 1, minting Rejoined proof token.",
            "Host LifecycleStore forces 4× RUNNING to superblocks. Re-crash before this re-classified dirty.",
            "uVRR delivers diskless throughput, provable safety, and zero amnesia by construction!"
        ],
        "speaker": "ALEX (HOST) & DR. MORGAN",
        "quote": "\"Diskless speed on normal path, mathematically sound crash-stop, guaranteed by Lean proofs!\""
    }
]

def main():
    FRAMES_DIR.mkdir(parents=True, exist_ok=True)
    cues_data = json.loads(CUES_JSON.read_text())
    
    print("=== Rendering 8 High-Definition Video Keyframes ===")
    frame_files = []
    durations = []
    for s_meta, s_cue in zip(STAGES_METADATA, cues_data["stages"]):
        dur = s_cue["end_time"] - s_cue["start_time"]
        f_path = FRAMES_DIR / f"frame_stage_{s_meta['num']}.png"
        render_frame(s_meta, f_path)
        frame_files.append(f_path)
        durations.append(dur)
        
    # Build ffmpeg concat script
    concat_txt = FRAMES_DIR / "concat_frames.txt"
    with open(concat_txt, "w") as f:
        for f_path, dur in zip(frame_files, durations):
            f.write(f"file '{f_path.resolve()}'\n")
            f.write(f"duration {dur:.3f}\n")
        # Repeat last frame
        f.write(f"file '{frame_files[-1].resolve()}'\n")
        
    print(f"Compositing MP4 video with ffmpeg -> {OUTPUT_MP4}...")
    cmd = [
        "ffmpeg", "-y",
        "-f", "concat", "-safe", "0", "-i", str(concat_txt),
        "-i", str(AUDIO_MP3),
        "-c:v", "libx264", "-pix_fmt", "yuv420p", "-r", "30",
        "-c:a", "aac", "-b:a", "192k",
        "-shortest",
        str(OUTPUT_MP4)
    ]
    subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    
    v_size = OUTPUT_MP4.stat().st_size
    print(f"=== Video Successfully Generated ===")
    print(f"Path: {OUTPUT_MP4} ({v_size / (1024*1024):.2f} MB)")

if __name__ == "__main__":
    main()
