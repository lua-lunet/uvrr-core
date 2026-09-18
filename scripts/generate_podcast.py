#!/usr/bin/env python3
"""
Generate two-voice podcast audio for uVRR Explainer using Mistral Voxtral TTS.
Stitches audio segments with ffmpeg and generates timestamp cues for animation sync.
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
AUDIT_LOG = ANIM_DIR / "audit.log"

VOICE_ALEX = "e3596645-b1af-469e-b857-f18ddedc7652"   # Oliver - British Male (Alex)
VOICE_MORGAN = "a3e41ea8-020b-44c0-8d8b-f6cc03524e31" # Jane - British Female (Dr. Morgan)
MODEL_TTS = "voxtral-mini-tts-latest"

def get_api_key():
    env_file = ROOT / ".env"
    if env_file.exists():
        for line in env_file.read_text().splitlines():
            if line.startswith("MISTRAL_API_KEY="):
                return line.split("=", 1)[1].strip().strip('"').strip("'")
    return os.environ.get("MISTRAL_API_KEY")

DIALOGUE = [
    # Stage 1
    (1, "Alex", "Welcome back to Distributed Systems Deep Dives! Today we are digging into uVRR—Unbounded Viewstamped Replication Revisited. We are looking at a classic three-node cluster: node zero is the elected leader, node one and two are backups, running in Era one with unit weights. Everything is humming along on the fast path without touching the disk."),
    (1, "Dr. Morgan", "Exactly, Alex. Notice that like classic VRR-2012, normal operations require zero disk writes on the critical path. Consensus is held strictly in volatile quorum memory. But the profound question has always been: what happens when a node suffers a sudden, unannounced crash?"),
    
    # Stage 2
    (2, "Alex", "Boom! There goes Node Two. A kernel panic, power trip, or SIGKILL. In legacy systems, this is where panic sets in, because Node Two just lost all its in-memory promises."),
    (2, "Dr. Morgan", "Right. In 2016, Michael, Ports, Sharma, and Szekeres proved that classic diskless VRR had a catastrophic blind spot here: an amnesiac replica could restart, forget a prior view-change promise, and cause a split brain. uVRR completely eliminates that class of bugs with one foundational rule: a crash is final for the protocol identity!"),
    
    # Stage 3
    (3, "Alex", "So Node Two is booting back up. But before it touches the network, it reads its local storage. It does not look at a single file—it reads four spaced superblock copies, just like TigerBeetle DB does!"),
    (3, "Dr. Morgan", "Precisely. Because disk writes tear, you cannot rely on two states. uVRR uses three states: RUNNING, STOPPED, and FLUSHED. If the node had done a controlled halt, all four copies would read FLUSHED. Here, the quorum read sees no FLUSHED mark. The host instantly knows: this node died dirty. It classifies the start as a crash. And crucially, it pays zero disk latency on boot—time is of the essence!"),
    
    # Stage 4
    (4, "Alex", "Notice that Node Two did not resume. Resuming an old crashed identity is strictly unrepresentable. Instead, it reincarnates under a brand-new identity: Node Two Prime!"),
    (4, "Dr. Morgan", "Yes! A newborn has made no prior promises, so amnesia is harmless by construction. Furthermore, Node Two Prime boots at Era zero. The genesis era of the cluster is Era one. By construction, uVRR engines drop in-cluster sync messages for an Era zero node. It cannot be fenced forward by internal view traffic."),
    
    # Stage 5
    (5, "Alex", "So how does Node Two Prime find where the cluster is without corrupting the active view?"),
    (5, "Dr. Morgan", "Through an outer gossip protocol that sits completely outside the consensus engine! Node Two Prime broadcasts a join gossip packet containing its known frontier—slot zero. Any node that hears it reports back the current cluster era and configuration."),
    
    # Stage 6
    (6, "Alex", "Node Zero—the leader—immediately adds Two Prime to its gossip-witness list! And look at that data stream: the leader pushes every missed log entry and commit directly to the witness!"),
    (6, "Dr. Morgan", "This is the heart of the equivalence theorem: gossip equals recovery, with the memory precondition removed. The leader's stream is self-certifying. Under Lean theorem H1 through H5, accepting a Phase-2 at view v raises the node's promise floor to v via implicit promise telescoping. Node Two Prime is completely caught up in as little as one round trip!"),
    
    # Stage 7
    (7, "Alex", "Now Node Two Prime is caught up, but it is still a weight-zero witness. To become a voting member, the leader executes a two-step reconfiguration sequence."),
    (7, "Dr. Morgan", "That's Turner's leader overlap in action. In Era two, the dead Node Two drops to weight zero while Two Prime joins as a standby. Then in Era three, Two Prime steps up to weight one while the old identity leaves permanently. Every consecutive era maintains overlapping majorities through the leader's casting vote. Zero stalls, zero safety violations!"),
    
    # Stage 8
    (8, "Alex", "And the final beauty of the design: once Node Two Prime is officially seated as a normal voting member, its engine mints a proof token that finally allows the host to write the deferred RUNNING latch to disk!"),
    (8, "Dr. Morgan", "Exactly. The disk write was kept off the critical path until the node was fully operational. A crash during catch-up would have re-classified as crashed with zero penalty. That is uVRR: diskless speed on the normal path, mathematically sound crash-stop reincarnation, and rock-solid safety guaranteed by Lean kernel-checked proofs!"),
    (8, "Alex", "Brilliant engineering. Check out the interactive slider below to step through each phase yourself!")
]

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
    with urllib.request.urlopen(req, timeout=60) as resp:
        data = json.loads(resp.read().decode("utf-8"))
        return base64.b64decode(data["audio_data"])

def get_audio_duration(file_path):
    cmd = [
        "ffprobe", "-v", "error", "-show_entries", "format=duration",
        "-of", "default=noprint_wrappers=1:nokey=1", str(file_path)
    ]
    res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=True)
    return float(res.stdout.strip())

def main():
    api_key = get_api_key()
    if not api_key:
        print("ERROR: MISTRAL_API_KEY not found in .env or environment", file=sys.stderr)
        sys.exit(1)
        
    ANIM_DIR.mkdir(parents=True, exist_ok=True)
    AUDIO_TMP.mkdir(parents=True, exist_ok=True)
    
    print(f"=== Synthesizing {len(DIALOGUE)} dialogue segments via Mistral Voxtral TTS ===")
    
    chunk_files = []
    chunk_durations = []
    stage_timing = {}
    
    current_time = 0.0
    current_stage = None
    
    audit_lines = [
        "=== uVRR Educational Podcast Generation Audit Log ===",
        f"TTS Model: {MODEL_TTS}",
        f"Host Voice (Alex): {VOICE_ALEX}",
        f"Expert Voice (Dr. Morgan): {VOICE_MORGAN}",
        f"Total Dialogues: {len(DIALOGUE)}",
        "--------------------------------------------------"
    ]
    
    for idx, (stage, speaker, text) in enumerate(DIALOGUE, 1):
        voice = VOICE_ALEX if speaker == "Alex" else VOICE_MORGAN
        chunk_path = AUDIO_TMP / f"chunk_{idx:02d}_{speaker}_{stage}.mp3"
        
        if not chunk_path.exists() or chunk_path.stat().st_size == 0:
            print(f"[{idx}/{len(DIALOGUE)}] Generating Stage {stage} ({speaker})...")
            audio_bytes = synthesize_speech(text, voice, api_key)
            chunk_path.write_bytes(audio_bytes)
        else:
            print(f"[{idx}/{len(DIALOGUE)}] Using cached Stage {stage} ({speaker})...")
            
        dur = get_audio_duration(chunk_path)
        chunk_files.append(chunk_path)
        chunk_durations.append(dur)
        
        if stage != current_stage:
            current_stage = stage
            stage_timing[stage] = {
                "start": round(current_time, 2),
                "title": f"Stage {stage}"
            }
        
        stage_timing[stage]["end"] = round(current_time + dur, 2)
        audit_lines.append(f"[{current_time:6.2f}s - {current_time+dur:6.2f}s] Stage {stage} | {speaker}: {text[:60]}... ({dur:.2f}s)")
        current_time += dur

    # Concat list file
    concat_list = AUDIO_TMP / "concat_list.txt"
    with open(concat_list, "w") as f:
        for p in chunk_files:
            f.write(f"file '{p.resolve()}'\n")
            
    print(f"Concatenating {len(chunk_files)} segments to {OUTPUT_MP3} via ffmpeg...")
    subprocess.run([
        "ffmpeg", "-y", "-f", "concat", "-safe", "0",
        "-i", str(concat_list),
        "-c", "copy", str(OUTPUT_MP3)
    ], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    
    total_dur = get_audio_duration(OUTPUT_MP3)
    print(f"Final MP3 produced: {OUTPUT_MP3} (Duration: {total_dur:.2f}s)")
    
    # Stage cues format
    stage_titles = {
        1: "Stage 1: Steady-State Replication at Era 1",
        2: "Stage 2: Node 2 Crashes (Power Loss / SIGKILL)",
        3: "Stage 3: Superblock Quorum Read at Boot Gate",
        4: "Stage 4: Blank Boot at Era 0 & Reincarnation as N2'",
        5: "Stage 5: Outer Join Gossip Broadcast",
        6: "Stage 6: Leader Witness Stream & Telescoping",
        7: "Stage 7: Two-Step Reconfiguration via Leader Overlap",
        8: "Stage 8: Seated Observation & Deferred RUNNING Latch"
    }
    
    cues_data = {
        "audio_file": "podcast.mp3",
        "total_duration": round(total_dur, 2),
        "stages": []
    }
    
    for stage_num in range(1, 9):
        st = stage_timing.get(stage_num, {"start": 0.0, "end": 0.0})
        cues_data["stages"].append({
            "stage": stage_num,
            "title": stage_titles[stage_num],
            "start_time": st["start"],
            "end_time": st["end"]
        })
        
    CUES_JSON.write_text(json.dumps(cues_data, indent=2))
    print(f"Wrote timestamp cues to {CUES_JSON}")
    
    audit_lines.append("--------------------------------------------------")
    audit_lines.append(f"Total Stitched Audio Duration: {total_dur:.2f}s")
    audit_lines.append(f"Output File: {OUTPUT_MP3}")
    AUDIT_LOG.write_text("\n".join(audit_lines))
    print(f"Wrote audit log to {AUDIT_LOG}")

if __name__ == "__main__":
    main()
