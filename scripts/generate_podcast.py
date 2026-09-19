#!/usr/bin/env python3
"""
Generate single-voice narration audio for the uVRR consistency-ladder
explainer using Mistral Voxtral TTS. One clip per ladder rung; clips are
bookmarks. Writes one stitched track (podcast.mp3), the clip cue sheet in
two forms — cues.json for tooling and cues.js for the player over file:// —
and an audit log.
"""

import os
import sys
import json
import base64
import urllib.request
import urllib.error
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ANIM_DIR = ROOT / "docs/animation"
AUDIO_TMP = ANIM_DIR / "audio_chunks"
OUTPUT_MP3 = ANIM_DIR / "podcast.mp3"
CUES_JSON = ANIM_DIR / "cues.json"
CUES_JS = ANIM_DIR / "cues.js"
AUDIT_LOG = ANIM_DIR / "audit.log"

VOICE_NARRATOR = "a3e41ea8-020b-44c0-8d8b-f6cc03524e31"  # Jane - British Female
MODEL_TTS = "voxtral-mini-tts-latest"

# Inter-clip silence: a clean beat between bookmarks, long enough that a
# skip lands unambiguously inside the next clip.
GAP_SECONDS = 0.6

# The ladder, one clip per rung. (id, bookmark title, narration)
CLIPS = [
    ("one-node",
     "One node: your data, one machine",
     "Start with the simplest thing that can possibly work: one machine, holding your data, answering your requests. It works — until the day it doesn't. The power fails, the disk dies, the kernel panics. In one moment, everything you stored is gone, and every request goes unanswered. Resilience is not an optimisation you add later; it is the first requirement. So we need more than one machine. The question is: how many, and how do they agree?"),
    ("split-brain",
     "Two nodes: the split brain",
     "The obvious answer is two: keep a mirror, copy every write to both. But now the two machines must agree, and the wire between them can break. When it does, each side carries on alone, certain it is the whole system. Ask a question, and you can get two different answers — two histories of the same truth. This is the split brain, and it is the central problem of replication. A mirror does not remove the failure; it duplicates the authority. Agreement needs something stronger than copying."),
    ("majority",
     "Three nodes: the majority",
     "Add a third machine, and something remarkable appears: the majority. Any two of the three overlap, so any decision that two nodes have made is a decision the whole system must remember. One machine can die, and its memory is not lost — it survives in the pair that remain. And the cloud was built for exactly this shape: three regions, each with its own power and its own networking, joined by fast interconnects. Three is not just a number; it is the smallest system that can lose a part and keep its mind."),
    ("arithmetic",
     "The arithmetic: 2F+1",
     "Here is the arithmetic that governs every replication system. To tolerate F failed machines, you need two F plus one: with five nodes you can lose two; with three, you can lose one. A majority must always survive, because the majority is the memory of the system — the place where every committed decision lives. Fewer than a majority, and the system must stop and wait: refusing to answer is safe; answering from a stale memory is not."),
    ("five-then-three",
     "Five, then back to three",
     "So use five, and lose any two — the service never notices. But five is a lot of machines to watch in an explainer, so from here on we draw three, and everything we show scales by the same arithmetic. Three nodes, one majority, one shared truth. Now: how do the three actually work together? That is the protocol — Viewstamped Replication."),
    ("views",
     "Views and the primary",
     "Viewstamped Replication organises time into views. In each view, one node is the primary — the leader — and the others are backups. The primary decides the order of every operation; the backups follow. There is no lock, no lease file on disk: leadership is a fact the majority holds in memory, and it is numbered by the view. When the view changes, the leadership can move — cleanly, and by agreement."),
    ("normal-path",
     "Normal operation: two round trips, no disk",
     "A client sends a request to the primary. The primary appends it to the log and sends a prepare to the backups. Each backup appends and acknowledges, and when the majority — the primary plus the backups — has answered, the operation is committed: the primary sends the commit, and every node applies it in order. Two network round trips, and not one disk write on the path. The speed of the protocol comes from what it refuses to do: it does not wait for a disk; it trusts the majority's memory."),
    ("failover",
     "Failover: the view change",
     "Now the primary itself dies. The backups stop hearing its heartbeat, and suspicion accrues. The surviving pair — still a majority — runs a view change: they exchange what they know, elect a new primary, and install the next view. Because the pair overlaps every past majority, nothing committed is forgotten; the new primary continues the log exactly where the old one left it. The failover costs round trips, not disk flushes — the memory that matters was never on one machine."),
]


def get_api_key():
    env_file = ROOT / ".env"
    if env_file.exists():
        for line in env_file.read_text().splitlines():
            if line.startswith("MISTRAL_API_KEY="):
                return line.split("=", 1)[1].strip().strip('"').strip("'")
    return os.environ.get("MISTRAL_API_KEY")


def synthesize_speech(text, voice_id, api_key):
    body = json.dumps({
        "model": MODEL_TTS,
        "input": text,
        "voice_id": voice_id,
        "response_format": "mp3"
    }).encode("utf-8")

    req = urllib.request.Request(
        "https://api.mistral.ai/v1/audio/speech",
        data=body,
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json"
        }
    )
    with urllib.request.urlopen(req, timeout=120) as resp:
        data = json.loads(resp.read().decode("utf-8"))
        return base64.b64decode(data["audio_data"])


def get_audio_duration(file_path):
    """Decode-accurate duration: MP3 format=duration is a bitrate estimate
    (the Voxtral chunks' estimates run ~8% short), and every estimate error
    becomes bookmark drift in the stitched timeline. Count decoded samples."""
    res = subprocess.run(
        ["ffmpeg", "-i", str(file_path), "-map", "0:a", "-f", "s16le", "-"],
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=True)
    samples = len(res.stdout) // 2  # s16le, mono
    return samples / probe_sample_rate(file_path)


def make_gap_clip(seconds, sample_rate):
    """One silent mp3 at the chunks' native rate, generated once and reused."""
    gap_path = AUDIO_TMP / f"gap_{int(seconds * 1000)}ms.mp3"
    if not gap_path.exists():
        subprocess.run([
            "ffmpeg", "-y", "-f", "lavfi",
            "-i", f"anullsrc=r={sample_rate}:cl=mono",
            "-t", str(seconds), "-q:a", "9", str(gap_path)
        ], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return gap_path


def probe_sample_rate(path):
    res = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "stream=sample_rate",
         "-of", "default=noprint_wrappers=1:nokey=1", str(path)],
        stdout=subprocess.PIPE, text=True, check=True)
    return int(res.stdout.strip().splitlines()[0])


def main():
    api_key = get_api_key()
    if not api_key:
        print("ERROR: MISTRAL_API_KEY not found in .env or environment",
              file=sys.stderr)
        sys.exit(1)

    ANIM_DIR.mkdir(parents=True, exist_ok=True)
    AUDIO_TMP.mkdir(parents=True, exist_ok=True)

    print(f"=== Synthesizing {len(CLIPS)} narration clips, "
          f"single voice (Jane), {MODEL_TTS} ===")

    # All TTS chunks share one native sample rate; the gap clip must match it
    # or a packet-concat stitch distorts the timeline (measured: a 44.1 kHz
    # gap inside a 22.05 kHz stream stretched 261 s of audio to 282 s).
    first = AUDIO_TMP / f"clip_01_{CLIPS[0][0]}.mp3"
    if not (first.exists() and first.stat().st_size > 0):
        first.write_bytes(synthesize_speech(CLIPS[0][2], VOICE_NARRATOR, api_key))
    sample_rate = probe_sample_rate(first)
    gap_clip = make_gap_clip(GAP_SECONDS, sample_rate)
    gap_dur = get_audio_duration(gap_clip)

    concat_files = []
    clips_out = []
    current_time = 0.0

    audit_lines = [
        "=== uVRR Consistency-Ladder Narration Generation Audit Log ===",
        f"TTS Model: {MODEL_TTS}",
        f"Narrator Voice (Jane, British Female): {VOICE_NARRATOR}",
        f"Clips: {len(CLIPS)}   Inter-clip gap: {GAP_SECONDS}s",
        "--------------------------------------------------"
    ]

    for idx, (clip_id, title, text) in enumerate(CLIPS, 1):
        chunk_path = AUDIO_TMP / f"clip_{idx:02d}_{clip_id}.mp3"

        if not chunk_path.exists() or chunk_path.stat().st_size == 0:
            print(f"[{idx}/{len(CLIPS)}] Synthesizing: {title}")
            audio_bytes = synthesize_speech(text, VOICE_NARRATOR, api_key)
            chunk_path.write_bytes(audio_bytes)
        else:
            print(f"[{idx}/{len(CLIPS)}] Using cached: {title}")

        dur = get_audio_duration(chunk_path)
        clips_out.append({
            "id": clip_id,
            "title": title,
            "start_time": round(current_time, 2),
            "end_time": round(current_time + dur, 2),
            "text": text,
        })
        audit_lines.append(
            f"[{current_time:7.2f}s - {current_time + dur:7.2f}s] "
            f"{clip_id}: {title} ({dur:.2f}s)")

        concat_files.append(chunk_path)
        current_time += dur
        if idx < len(CLIPS):
            concat_files.append(gap_clip)
            current_time += gap_dur

    concat_list = AUDIO_TMP / "concat_list.txt"
    with open(concat_list, "w") as f:
        for p in concat_files:
            f.write(f"file '{p.resolve()}'\n")

    # Sample-accurate stitch: decode every input and re-encode once, so a
    # mixed-rate packet concat can never shift the timeline against cues.
    print(f"Stitching {len(concat_files)} segments to {OUTPUT_MP3} "
          f"(decode + single re-encode)...")
    inputs = []
    for p in concat_files:
        inputs.extend(["-i", str(p)])
    n = len(concat_files)
    filt = "".join(f"[{i}:a]" for i in range(n)) + f"concat=n={n}:v=0:a=1[out]"
    subprocess.run(
        ["ffmpeg", "-y", *inputs, "-filter_complex", filt,
         "-map", "[out]", "-ar", str(sample_rate), "-ac", "1",
         "-b:a", "64k", str(OUTPUT_MP3)],
        check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    total_dur = get_audio_duration(OUTPUT_MP3)
    print(f"Narration track: {OUTPUT_MP3} (duration {total_dur:.2f}s)")

    cues_data = {
        "audio_file": "podcast.mp3",
        "total_duration": round(total_dur, 2),
        "clips": clips_out,
    }
    CUES_JSON.write_text(json.dumps(cues_data, indent=2))
    CUES_JS.write_text(
        "// Generated by scripts/generate_podcast.py — do not edit.\n"
        f"window.UVRR_CUES = {json.dumps(cues_data, indent=2)};\n"
    )
    print(f"Wrote cues to {CUES_JSON} and {CUES_JS}")

    audit_lines.append("--------------------------------------------------")
    audit_lines.append(f"Total stitched duration: {total_dur:.2f}s")
    audit_lines.append(f"Output: {OUTPUT_MP3}")
    AUDIT_LOG.write_text("\n".join(audit_lines))
    print(f"Wrote audit log to {AUDIT_LOG}")


if __name__ == "__main__":
    main()
