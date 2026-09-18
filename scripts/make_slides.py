#!/usr/bin/env python3
"""
Generate presentation deck for uVRR Controlled Lifecycle, Boot Gate, and Gossip Equivalence.
Conforms to British English and rigorous engineering documentation standards.
"""

import sys
import os
from pptx import Presentation
from pptx.util import Inches, Pt
from pptx.dml.color import RGBColor
from pptx.enum.text import PP_ALIGN, MSO_ANCHOR
from pptx.enum.shapes import MSO_SHAPE

def build_deck(output_pptx_path):
    prs = Presentation()
    prs.slide_width = Inches(13.333)
    prs.slide_height = Inches(7.5)
    blank_layout = prs.slide_layouts[6]

    # Style Tokens
    C_NAVY_DARK = RGBColor(15, 23, 42)      # #0F172A
    C_NAVY_CARD = RGBColor(30, 41, 59)      # #1E293B
    C_SLATE_BORDER = RGBColor(51, 65, 85)   # #334155
    C_BG_LIGHT = RGBColor(248, 250, 252)    # #F8FAFC
    C_CARD_BG = RGBColor(255, 255, 255)     # #FFFFFF
    C_CARD_BORDER = RGBColor(226, 232, 240) # #E2E8F0
    C_TEXT_DARK = RGBColor(15, 23, 42)      # #0F172A
    C_TEXT_MUTED = RGBColor(100, 116, 139)  # #64748B
    C_BLUE = RGBColor(37, 99, 235)          # #2563EB
    C_BLUE_LIGHT = RGBColor(239, 246, 255)  # #EFF6FF
    C_AMBER = RGBColor(217, 119, 6)         # #D97706
    C_AMBER_LIGHT = RGBColor(254, 243, 199) # #FEF3C7
    C_GREEN = RGBColor(22, 163, 74)         # #16A34A
    C_GREEN_LIGHT = RGBColor(240, 253, 244) # #F0FDF4
    C_RED = RGBColor(220, 38, 38)           # #DC2626
    C_RED_LIGHT = RGBColor(254, 242, 242)   # #FEF2F2
    C_PURPLE = RGBColor(126, 34, 206)       # #7E22CE
    C_PURPLE_LIGHT = RGBColor(250, 245, 255)# #FAF5FF

    def add_header(slide, category, title):
        # Category pill / banner
        cat_box = slide.shapes.add_textbox(Inches(0.8), Inches(0.45), Inches(11.7), Inches(0.35))
        tf_cat = cat_box.text_frame
        tf_cat.word_wrap = True
        tf_cat.margin_left = tf_cat.margin_right = tf_cat.margin_top = tf_cat.margin_bottom = 0
        p_cat = tf_cat.paragraphs[0]
        p_cat.text = category.upper()
        p_cat.font.size = Pt(10)
        p_cat.font.bold = True
        p_cat.font.color.rgb = C_BLUE

        # Slide Title
        title_box = slide.shapes.add_textbox(Inches(0.8), Inches(0.75), Inches(11.7), Inches(0.6))
        tf_title = title_box.text_frame
        tf_title.word_wrap = True
        tf_title.margin_left = tf_title.margin_right = tf_title.margin_top = tf_title.margin_bottom = 0
        p_title = tf_title.paragraphs[0]
        p_title.text = title
        p_title.font.size = Pt(22)
        p_title.font.bold = True
        p_title.font.color.rgb = C_TEXT_DARK

    def add_footer(slide, current_slide, total_slides=9, source="docs/uvrr-boot-gate.md"):
        footer_box = slide.shapes.add_textbox(Inches(0.8), Inches(7.0), Inches(11.7), Inches(0.3))
        tf = footer_box.text_frame
        tf.word_wrap = True
        tf.margin_left = tf.margin_right = tf.margin_top = tf.margin_bottom = 0
        p = tf.paragraphs[0]
        p.text = f"uVRR Formal Specification  |  Source: {source}  |  Slide {current_slide} of {total_slides}"
        p.font.size = Pt(9)
        p.font.color.rgb = C_TEXT_MUTED

    def create_card(slide, left, top, width, height, bg_color=C_CARD_BG, border_color=C_CARD_BORDER):
        shape = slide.shapes.add_shape(MSO_SHAPE.RECTANGLE, left, top, width, height)
        shape.fill.solid()
        shape.fill.fore_color.rgb = bg_color
        shape.line.color.rgb = border_color
        shape.line.width = Pt(1)
        return shape

    # =========================================================================
    # SLIDE 1: Title Slide (Dark Theme)
    # =========================================================================
    s1 = prs.slides.add_slide(blank_layout)
    bg1 = s1.shapes.add_shape(MSO_SHAPE.RECTANGLE, 0, 0, prs.slide_width, prs.slide_height)
    bg1.fill.solid()
    bg1.fill.fore_color.rgb = C_NAVY_DARK
    bg1.line.fill.background()

    # Title Card Accent
    acc1 = s1.shapes.add_shape(MSO_SHAPE.RECTANGLE, Inches(1.2), Inches(1.8), Inches(0.12), Inches(3.2))
    acc1.fill.solid()
    acc1.fill.fore_color.rgb = C_BLUE
    acc1.line.fill.background()

    tb1 = s1.shapes.add_textbox(Inches(1.5), Inches(1.7), Inches(10.5), Inches(3.5))
    tf1 = tb1.text_frame
    tf1.word_wrap = True

    p1_tag = tf1.paragraphs[0]
    p1_tag.text = "UVRR ARCHITECTURAL & FORMAL FOUNDATIONS"
    p1_tag.font.size = Pt(12)
    p1_tag.font.bold = True
    p1_tag.font.color.rgb = C_BLUE
    p1_tag.space_after = Pt(12)

    p1_title = tf1.add_paragraph()
    p1_title.text = "Controlled Halts, Boot Gates,\nand Rejoin Equivalence"
    p1_title.font.size = Pt(36)
    p1_title.font.bold = True
    p1_title.font.color.rgb = RGBColor(255, 255, 255)
    p1_title.space_after = Pt(16)

    p1_sub = tf1.add_paragraph()
    p1_sub.text = "Rigorous separation of clean halts from crashes, 3-state superblock disambiguation of torn writes, happens-before write schedules, and proving gossip ≡ recovery without memory."
    p1_sub.font.size = Pt(15)
    p1_sub.font.color.rgb = RGBColor(203, 213, 225)

    # Metadata pill
    tb1_meta = s1.shapes.add_textbox(Inches(1.5), Inches(5.6), Inches(10.5), Inches(0.8))
    tf1_meta = tb1_meta.text_frame
    p1_m = tf1_meta.paragraphs[0]
    p1_m.text = "Formal Target State  •  British English  •  Lean 4.33.1 & TigerBeetle Superblock Foundation\nReferences: docs/uvrr-boot-gate.md  |  research/trex-vrr-gossip-equivalence.md  |  paper.tex"
    p1_m.font.size = Pt(11)
    p1_m.font.color.rgb = RGBColor(148, 163, 184)

    s1.notes_slide.notes_text_frame.text = (
        "SOURCE: docs/uvrr-boot-gate.md, research/trex-vrr-gossip-equivalence.md, formal/uvrr-lean/paper/paper.tex.\n"
        "REPORTING BASIS: Formal protocol specification and kernel-checked invariants.\n"
        "PRIMARY MESSAGE: Clean stops and crashes are fundamentally distinct lifecycle transitions. "
        "Resume names nothing in uVRR. This slide deck establishes the exact state transitions, "
        "durable marker classification rules, write schedules, and the equivalence theorem resolving gossip against VRR-2012 recovery."
    )

    # =========================================================================
    # SLIDE 2: Definitional Separation: Halt vs Crash
    # =========================================================================
    s2 = prs.slides.add_slide(blank_layout)
    add_header(s2, "Lifecycle Classification", "A Controlled Halt is Not a Crash; A Controlled Start is Not a Recovery")
    add_footer(s2, 2, 9, "docs/uvrr-boot-gate.md §1")

    # Banner callout
    b2 = create_card(s2, Inches(0.8), Inches(1.45), Inches(11.733), Inches(0.8), bg_color=C_BLUE_LIGHT, border_color=C_BLUE)
    tb2_b = s2.shapes.add_textbox(Inches(1.0), Inches(1.5), Inches(11.333), Inches(0.7))
    tf2_b = tb2_b.text_frame
    tf2_b.word_wrap = True
    p2_b = tf2_b.paragraphs[0]
    p2_b.text = "Core Rule: 'Resume' names nothing in uVRR. The resumption of an amnesiac crashed identity is an unrepresentable state to eliminate the Michael et al. DISC'17 view-change split-brain vulnerability."
    p2_b.font.size = Pt(13)
    p2_b.font.bold = True
    p2_b.font.color.rgb = C_BLUE

    # Card 1: Ordinary Controlled Lifecycle
    create_card(s2, Inches(0.8), Inches(2.45), Inches(5.7), Inches(4.3), bg_color=C_CARD_BG, border_color=C_CARD_BORDER)
    acc2_1 = s2.shapes.add_shape(MSO_SHAPE.RECTANGLE, Inches(0.8), Inches(2.45), Inches(5.7), Inches(0.08))
    acc2_1.fill.solid()
    acc2_1.fill.fore_color.rgb = C_GREEN
    acc2_1.line.fill.background()

    tb2_c1 = s2.shapes.add_textbox(Inches(1.05), Inches(2.65), Inches(5.2), Inches(3.9))
    tf2_c1 = tb2_c1.text_frame
    tf2_c1.word_wrap = True
    
    p = tf2_c1.paragraphs[0]
    p.text = "Ordinary Lifecycle: Run → Halt → Run"
    p.font.size = Pt(16)
    p.font.bold = True
    p.font.color.rgb = C_GREEN
    p.space_after = Pt(10)

    p = tf2_c1.add_paragraph()
    p.text = "• Controlled Halt: The host suspends the read loop, drains pending handlers, forces the dual-ring WAL, and records the flush on disk."
    p.font.size = Pt(13)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(8)

    p = tf2_c1.add_paragraph()
    p.text = "• Controlled Start: The process boots over vouched durable state. It continues from its recorded era. No recovery protocol executes."
    p.font.size = Pt(13)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(8)

    p = tf2_c1.add_paragraph()
    p.text = "• Cluster Invariant: The cluster served a quorum throughout the absence. The node re-synchronises in-cluster exactly as though a network partition healed."
    p.font.size = Pt(13)
    p.font.color.rgb = C_TEXT_DARK

    # Card 2: Crashed Lifecycle
    create_card(s2, Inches(6.833), Inches(2.45), Inches(5.7), Inches(4.3), bg_color=C_CARD_BG, border_color=C_CARD_BORDER)
    acc2_2 = s2.shapes.add_shape(MSO_SHAPE.RECTANGLE, Inches(6.833), Inches(2.45), Inches(5.7), Inches(0.08))
    acc2_2.fill.solid()
    acc2_2.fill.fore_color.rgb = C_AMBER
    acc2_2.line.fill.background()

    tb2_c2 = s2.shapes.add_textbox(Inches(7.083), Inches(2.65), Inches(5.2), Inches(3.9))
    tf2_c2 = tb2_c2.text_frame
    tf2_c2.word_wrap = True

    p = tf2_c2.paragraphs[0]
    p.text = "Crashed Lifecycle: Crash → Stop → Reincarnate"
    p.font.size = Pt(16)
    p.font.bold = True
    p.font.color.rgb = C_AMBER
    p.space_after = Pt(10)

    p = tf2_c2.add_paragraph()
    p.text = "• Finality of Crash: A crash is final for the protocol identity. The old node identity is permanently retired and will never speak again."
    p.font.size = Pt(13)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(8)

    p = tf2_c2.add_paragraph()
    p.text = "• Crash-Stop Self-Eviction: The replacement process initialises with a fresh bumped incarnation (e.g. N0 → N0'). It has made no promises and owns no votes."
    p.font.size = Pt(13)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(8)

    p = tf2_c2.add_paragraph()
    p.text = "• Reconfiguration Authority: Authority is seated solely through a committed multi-step configuration sequence (Turner leader overlap), never via memory."
    p.font.size = Pt(13)
    p.font.color.rgb = C_TEXT_DARK

    s2.notes_slide.notes_text_frame.text = (
        "SOURCE: docs/uvrr-boot-gate.md §1, docs/uvrr-reincarnation.md §1.\n"
        "CALCULATION BASIS: Protocol state-transition algebra.\n"
        "PRIMARY MESSAGE: The terminology is strictly enforced: clean stop is not a crash, and clean start is not a recovery. "
        "Classic VRR recovery allows a node to return under its old identity, which Michael et al. showed leads to linearizability violations "
        "if promises made before the crash are forgotten. uVRR eliminates this entirely by making a crash final for the identity."
    )

    # =========================================================================
    # SLIDE 3: Why Three Marker States: Disambiguating Torn Writes
    # =========================================================================
    s3 = prs.slides.add_slide(blank_layout)
    add_header(s3, "Durability Primitives", "Three Marker States Log Transition Boundaries to Disambiguate Torn Writes")
    add_footer(s3, 3, 9, "docs/uvrr-boot-gate.md §2")

    # Left Explanation
    create_card(s3, Inches(0.8), Inches(1.45), Inches(4.3), Inches(5.3), bg_color=C_CARD_BG, border_color=C_CARD_BORDER)
    tb3_l = s3.shapes.add_textbox(Inches(1.0), Inches(1.65), Inches(3.9), Inches(4.9))
    tf3_l = tb3_l.text_frame
    tf3_l.word_wrap = True

    p = tf3_l.paragraphs[0]
    p.text = "The Ambiguity of Two States"
    p.font.size = Pt(16)
    p.font.bold = True
    p.font.color.rgb = C_NAVY_DARK
    p.space_after = Pt(10)

    p = tf3_l.add_paragraph()
    p.text = "• Spaced Writes Can Tear: A marker is written across four independent disk blocks. A power loss or kernel panic mid-write leaves a torn mixture of copies."
    p.font.size = Pt(12)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(8)

    p = tf3_l.add_paragraph()
    p.text = "• The Two-State Failure: If disk holds only RUNNING and STOPPED, a mixture reveals that a write tore, but the reader cannot tell if the node died starting up or halting down."
    p.font.size = Pt(12)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(8)

    p = tf3_l.add_paragraph()
    p.text = "• Three-State Solution: Logging the beginning AND the end of every transition removes all ambiguity:"
    p.font.size = Pt(12)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(4)

    p = tf3_l.add_paragraph()
    p.text = "   RUNNING ──stop──> STOPPED ──drain──> FLUSHED"
    p.font.size = Pt(11)
    p.font.bold = True
    p.font.color.rgb = C_BLUE
    p.space_after = Pt(8)

    p = tf3_l.add_paragraph()
    p.text = "• Minimum Progress Principle: A crash during marker writes must never falsely classify as a clean stop. The classifier takes the minimum progress observed."
    p.font.size = Pt(12)
    p.font.color.rgb = C_TEXT_DARK

    # Right: The Decision & Classification Table
    t_shape = s3.shapes.add_table(5, 3, Inches(5.4), Inches(1.45), Inches(7.133), Inches(5.3))
    table = t_shape.table
    table.columns[0].width = Inches(1.8)
    table.columns[1].width = Inches(2.3)
    table.columns[2].width = Inches(3.033)

    headers = ["MARKER STATE", "WRITTEN WHEN", "VOUCHES FOR"]
    for i, h in enumerate(headers):
        cell = table.cell(0, i)
        cell.fill.solid()
        cell.fill.fore_color.rgb = C_NAVY_DARK
        p = cell.text_frame.paragraphs[0]
        p.text = h
        p.font.size = Pt(12)
        p.font.bold = True
        p.font.color.rgb = RGBColor(255, 255, 255)

    data = [
        ("RUNNING", "At boot, before first message is processed off wire", "Nothing — process is live, state beneath is unvouched."),
        ("STOPPED", "When controlled halt begins; wire closed, handlers drained", "Nothing yet — durable WAL/grid flush is still in flight."),
        ("FLUSHED", "When host WAL and application state drain completes", "Everything beneath — verified zero amnesiac risk."),
        ("TORN MIXTURE\n(Classification)", "RUNNING + STOPPED: died halting (no FLUSHED quorum)\nSTOPPED + FLUSHED: died completing flush (FLUSHED vouches)", "RUNNING + STOPPED ≡ Crashed\nSTOPPED + FLUSHED ≡ Clean\nEvery mix classifies; none ambiguous.")
    ]

    for row_idx, row_data in enumerate(data, start=1):
        for col_idx, text in enumerate(row_data):
            cell = table.cell(row_idx, col_idx)
            cell.fill.solid()
            if row_idx == 4:
                cell.fill.fore_color.rgb = C_BLUE_LIGHT
            elif row_idx % 2 == 1:
                cell.fill.fore_color.rgb = RGBColor(255, 255, 255)
            else:
                cell.fill.fore_color.rgb = RGBColor(241, 245, 249)
            
            p = cell.text_frame.paragraphs[0]
            p.text = text
            p.font.size = Pt(11)
            p.font.color.rgb = C_TEXT_DARK
            if row_idx == 4 and col_idx == 0:
                p.font.bold = True

    s3.notes_slide.notes_text_frame.text = (
        "SOURCE: docs/uvrr-boot-gate.md §2, tests/lifecycle.rs.\n"
        "REPORTING PERIOD / BASIS: 12^4 = 20,736 exhaustive assignments of (identity, marker) across 4 copies tested in tests/lifecycle.rs.\n"
        "PRIMARY MESSAGE: Spaced writes tear across sector boundaries. With 2 states, a torn write is completely ambiguous. "
        "With 3 states (RUNNING, STOPPED, FLUSHED), both the start and end of transitions are durably bracketed. "
        "A quorum of FLUSHED marks guarantees the drain completed before the write. Any other mixture classifies as crashed."
    )

    # =========================================================================
    # SLIDE 4: The Three Write Schedules & Happens-Before Order
    # =========================================================================
    s4 = prs.slides.add_slide(blank_layout)
    add_header(s4, "Happens-Before Specification", "Asymmetric Write Budgets: Two Rounds Down, One Up, Zero on Dirty Start")
    add_footer(s4, 4, 9, "docs/uvrr-boot-gate.md §3")

    col_w = Inches(3.644)
    gap = Inches(0.4)

    # Column 1: Controlled Halt
    create_card(s4, Inches(0.8), Inches(1.45), col_w, Inches(5.3), bg_color=C_CARD_BG, border_color=C_CARD_BORDER)
    top_bar1 = s4.shapes.add_shape(MSO_SHAPE.RECTANGLE, Inches(0.8), Inches(1.45), col_w, Inches(0.08))
    top_bar1.fill.solid()
    top_bar1.fill.fore_color.rgb = C_BLUE
    top_bar1.line.fill.background()

    tb4_1 = s4.shapes.add_textbox(Inches(1.0), Inches(1.65), Inches(3.244), Inches(4.9))
    tf4_1 = tb4_1.text_frame
    tf4_1.word_wrap = True
    p = tf4_1.paragraphs[0]
    p.text = "1. Controlled Halt\n(Two Forced Rounds Down)"
    p.font.size = Pt(15)
    p.font.bold = True
    p.font.color.rgb = C_BLUE
    p.space_after = Pt(10)

    steps_1 = [
        "1. Quiesce Wire: Halt the read loop and drain all in-flight handlers.",
        "2. Round 1 Write: Force write 4× STOPPED markers to storage.",
        "3. Force WAL: Sync the dual-ring write-ahead log & state to disk.",
        "4. Round 2 Write: Force write 4× FLUSHED markers to superblocks.",
        "5. Clean Exit: Process exits. Shutdown monitor watches disk markers, not PID."
    ]
    for s in steps_1:
        p = tf4_1.add_paragraph()
        p.text = s
        p.font.size = Pt(11)
        p.font.color.rgb = C_TEXT_DARK
        p.space_after = Pt(6)

    # Column 2: Clean Start
    create_card(s4, Inches(0.8) + col_w + gap, Inches(1.45), col_w, Inches(5.3), bg_color=C_CARD_BG, border_color=C_CARD_BORDER)
    top_bar2 = s4.shapes.add_shape(MSO_SHAPE.RECTANGLE, Inches(0.8) + col_w + gap, Inches(1.45), col_w, Inches(0.08))
    top_bar2.fill.solid()
    top_bar2.fill.fore_color.rgb = C_GREEN
    top_bar2.line.fill.background()

    tb4_2 = s4.shapes.add_textbox(Inches(1.0) + col_w + gap, Inches(1.65), Inches(3.244), Inches(4.9))
    tf4_2 = tb4_2.text_frame
    tf4_2.word_wrap = True
    p = tf4_2.paragraphs[0]
    p.text = "2. Clean Start\n(One Forced Round Up)"
    p.font.size = Pt(15)
    p.font.bold = True
    p.font.color.rgb = C_GREEN
    p.space_after = Pt(10)

    steps_2 = [
        "1. Quorum Read: Read 4 superblock copies; confirm ≥2/4 copies read FLUSHED.",
        "2. Single Round Write: Force write 4× RUNNING as the boot-gate latch.",
        "3. Latch Purpose: Written before the first message is processed off wire so a subsequent crash is never misclassified.",
        "4. In-Cluster Sync: Read era from vouched disk state; re-sync in-cluster via standard partition-heal semantics."
    ]
    for s in steps_2:
        p = tf4_2.add_paragraph()
        p.text = s
        p.font.size = Pt(11)
        p.font.color.rgb = C_TEXT_DARK
        p.space_after = Pt(8)

    # Column 3: Dirty Fast Start
    create_card(s4, Inches(0.8) + (col_w + gap)*2, Inches(1.45), col_w, Inches(5.3), bg_color=C_CARD_BG, border_color=C_CARD_BORDER)
    top_bar3 = s4.shapes.add_shape(MSO_SHAPE.RECTANGLE, Inches(0.8) + (col_w + gap)*2, Inches(1.45), col_w, Inches(0.08))
    top_bar3.fill.solid()
    top_bar3.fill.fore_color.rgb = C_AMBER
    top_bar3.line.fill.background()

    tb4_3 = s4.shapes.add_textbox(Inches(1.0) + (col_w + gap)*2, Inches(1.65), Inches(3.244), Inches(4.9))
    tf4_3 = tb4_3.text_frame
    tf4_3.word_wrap = True
    p = tf4_3.paragraphs[0]
    p.text = "3. Dirty Fast Start\n(0 Rounds Now, 1 Later)"
    p.font.size = Pt(15)
    p.font.bold = True
    p.font.color.rgb = C_AMBER
    p.space_after = Pt(10)

    steps_3 = [
        "1. Quorum Read: Superblocks show no FLUSHED quorum (died uncleanly).",
        "2. Zero Disk Pause: Time is of the essence. Do NOT block on disk flush.",
        "3. Outer Gossip: Uninitialised node gossips frontiers to find cluster and catch up.",
        "4. Deferred Latch: The 4× RUNNING write is deferred until cluster seats the node.",
        "5. Safe by Construction: Re-crash before deferred latch still reads unvouched and classifies crashed again."
    ]
    for s in steps_3:
        p = tf4_3.add_paragraph()
        p.text = s
        p.font.size = Pt(11)
        p.font.color.rgb = C_TEXT_DARK
        p.space_after = Pt(6)

    s4.notes_slide.notes_text_frame.text = (
        "SOURCE: docs/uvrr-boot-gate.md §3, docs/diagrams/uvrr-boot-gate-sequence.svg.\n"
        "CALCULATION BASIS: Host I/O round trips and barrier placement.\n"
        "PRIMARY MESSAGE: Write schedules are asymmetric and fixed by the path. Controlled halt does 2 rounds down with WAL flush "
        "strictly between. Clean start does 1 round up to latch RUNNING. Dirty start pays zero disk flush at startup boundary; "
        "the latch is deferred until after cluster rejoin. The deferral is provably safe because re-crashing during deferral still reads unvouched."
    )

    # =========================================================================
    # SLIDE 5: The Era-0 Exclusion Rule by Construction
    # =========================================================================
    s5 = prs.slides.add_slide(blank_layout)
    add_header(s5, "Protocol Boundary Invariants", "Genesis Era is 1: Era 0 Admits Zero In-Cluster Synchronisation")
    add_footer(s5, 5, 9, "docs/uvrr-boot-gate.md §4")

    # Banner callout
    b5 = create_card(s5, Inches(0.8), Inches(1.45), Inches(11.733), Inches(0.85), bg_color=C_RED_LIGHT, border_color=C_RED)
    tb5_b = s5.shapes.add_textbox(Inches(1.0), Inches(1.5), Inches(11.333), Inches(0.75))
    tf5_b = tb5_b.text_frame
    tf5_b.word_wrap = True
    p5_b = tf5_b.paragraphs[0]
    p5_b.text = "Invariant Rule: By construction, there is NO in-cluster state synchronisation in uVRR to a node in era 0. Era 0 is uninitialised/pre-genesis; genesis era is 1. A node presenting era 0 has no cluster state to sync against."
    p5_b.font.size = Pt(13)
    p5_b.font.bold = True
    p5_b.font.color.rgb = C_RED

    # Card 1: Clean Start in Established Era
    create_card(s5, Inches(0.8), Inches(2.55), Inches(5.7), Inches(4.2), bg_color=C_CARD_BG, border_color=C_CARD_BORDER)
    acc5_1 = s5.shapes.add_shape(MSO_SHAPE.RECTANGLE, Inches(0.8), Inches(2.55), Inches(5.7), Inches(0.08))
    acc5_1.fill.solid()
    acc5_1.fill.fore_color.rgb = C_GREEN
    acc5_1.line.fill.background()

    tb5_c1 = s5.shapes.add_textbox(Inches(1.05), Inches(2.75), Inches(5.2), Inches(3.8))
    tf5_c1 = tb5_c1.text_frame
    tf5_c1.word_wrap = True
    p = tf5_c1.paragraphs[0]
    p.text = "Clean Start: Era ≥ 1 Vouched"
    p.font.size = Pt(16)
    p.font.bold = True
    p.font.color.rgb = C_GREEN
    p.space_after = Pt(10)

    p = tf5_c1.add_paragraph()
    p.text = "• Vouched Disk Read: Node reads its durable state (e.g. era = 1234) following a clean, FLUSHED-vouched halt."
    p.font.size = Pt(13)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(8)

    p = tf5_c1.add_paragraph()
    p.text = "• In-Cluster Sync Permitted: Because era ≥ 1 is proven, the node participates directly in the cluster's normal state synchronisation protocols."
    p.font.size = Pt(13)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(8)

    p = tf5_c1.add_paragraph()
    p.text = "• Zero Gossip Needed: The node is already recognised in the configuration history; the absence is treated as a transient network partition."
    p.font.size = Pt(13)
    p.font.color.rgb = C_TEXT_DARK

    # Card 2: Blank / Dirty Start at Era 0
    create_card(s5, Inches(6.833), Inches(2.55), Inches(5.7), Inches(4.2), bg_color=C_CARD_BG, border_color=C_CARD_BORDER)
    acc5_2 = s5.shapes.add_shape(MSO_SHAPE.RECTANGLE, Inches(6.833), Inches(2.55), Inches(5.7), Inches(0.08))
    acc5_2.fill.solid()
    acc5_2.fill.fore_color.rgb = C_BLUE
    acc5_2.line.fill.background()

    tb5_c2 = s5.shapes.add_textbox(Inches(7.083), Inches(2.75), Inches(5.2), Inches(3.8))
    tf5_c2 = tb5_c2.text_frame
    tf5_c2.word_wrap = True
    p = tf5_c2.paragraphs[0]
    p.text = "Blank Boot: Outer Gossip Discovery"
    p.font.size = Pt(16)
    p.font.bold = True
    p.font.color.rgb = C_BLUE
    p.space_after = Pt(10)

    p = tf5_c2.add_paragraph()
    p.text = "• Uninitialised Era 0: Following a crash, a node boots blank at era 0 and never remembers an era across the failure boundary."
    p.font.size = Pt(13)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(8)

    p = tf5_c2.add_paragraph()
    p.text = "• In-Cluster Sync Refused: The engine refuses to process incoming normal replication for era 0. The node cannot be fenced forward by VRR view traffic."
    p.font.size = Pt(13)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(8)

    p = tf5_c2.add_paragraph()
    p.text = "• Outer Gossip Adoption: The node must use external join gossip to discover the current cluster era and configuration before its engine sees wire traffic."
    p.font.size = Pt(13)
    p.font.color.rgb = C_TEXT_DARK

    s5.notes_slide.notes_text_frame.text = (
        "SOURCE: docs/uvrr-boot-gate.md §4, tests/configuration_contract.rs.\n"
        "CONTRACT: tests/configuration_contract.rs verifies that VOID occupies slot 1 and INIT establishes era 1.\n"
        "PRIMARY MESSAGE: In uVRR, genesis era is 1. Era 0 has no meaning in active consensus. "
        "A node that boots blank at era 0 cannot use VRR view-change or state sync to catch up because it has no view or era to be fenced from. "
        "This is why rejoin is designed as an outer gossip protocol that discovers the era before entering the engine."
    )

    # =========================================================================
    # SLIDE 6: Host Storage Obligations: Superblock Quorums
    # =========================================================================
    s6 = prs.slides.add_slide(blank_layout)
    add_header(s6, "Storage Architecture", "Host Durability Contract: Superblock Quorums and Direct I/O Barriers")
    add_footer(s6, 6, 9, "docs/uvrr-boot-gate.md §5, docs/uvrr-termination-obligations.md §4")

    # Left: The Storage Reality & Literature
    create_card(s6, Inches(0.8), Inches(1.45), Inches(5.7), Inches(5.3), bg_color=C_CARD_BG, border_color=C_CARD_BORDER)
    tb6_l = s6.shapes.add_textbox(Inches(1.05), Inches(1.65), Inches(5.2), Inches(4.9))
    tf6_l = tb6_l.text_frame
    tf6_l.word_wrap = True

    p = tf6_l.paragraphs[0]
    p.text = "The Perils of Single fsync() Flags"
    p.font.size = Pt(16)
    p.font.bold = True
    p.font.color.rgb = C_NAVY_DARK
    p.space_after = Pt(10)

    p = tf6_l.add_paragraph()
    p.text = "• File System Lies: Modern disk controllers and operating system page caches reorder writes. Single-file flags or markers are notoriously prone to silent corruption, misdirected writes, and torn sectors."
    p.font.size = Pt(12)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(8)

    p = tf6_l.add_paragraph()
    p.text = "• Grounded in Literature: Pillai et al. (OSDI 2014, 'All File Systems Are Not Created Equal') and Chidambaram et al. (SOSP 2013) proved that fsync() alone does not guarantee crash consistency across power loss."
    p.font.size = Pt(12)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(8)

    p = tf6_l.add_paragraph()
    p.text = "• Strict Host Obligation: A host that cannot guarantee classification from durable state is unsafe, full stop. The test harness implements crash-stop and validates error-on-crashed."
    p.font.size = Pt(12)
    p.font.color.rgb = C_TEXT_DARK

    # Right: TigerBeetle Superblock Techniques Borrowed
    create_card(s6, Inches(6.833), Inches(1.45), Inches(5.7), Inches(5.3), bg_color=C_CARD_BG, border_color=C_CARD_BORDER)
    acc6_r = s6.shapes.add_shape(MSO_SHAPE.RECTANGLE, Inches(6.833), Inches(1.45), Inches(5.7), Inches(0.08))
    acc6_r.fill.solid()
    acc6_r.fill.fore_color.rgb = C_BLUE
    acc6_r.line.fill.background()

    tb6_r = s6.shapes.add_textbox(Inches(7.083), Inches(1.65), Inches(5.2), Inches(4.9))
    tf6_r = tb6_r.text_frame
    tf6_r.word_wrap = True

    p = tf6_r.paragraphs[0]
    p.text = "TigerBeetle Storage Engineering"
    p.font.size = Pt(16)
    p.font.bold = True
    p.font.color.rgb = C_BLUE
    p.space_after = Pt(10)

    p = tf6_r.add_paragraph()
    p.text = "• 4-Copy Spaced Superblock: Exactly 4 copies spaced across disk zones, checksummed and sequence hash-chained (tigerbeetle/tigerbeetle)."
    p.font.size = Pt(12)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(8)

    p = tf6_r.add_paragraph()
    p.text = "• Quorum Reads (≥2/4 or ≥3/4): Resolves the highest-incarnation working cohort; automatically rewrites and repairs lagging or torn copies."
    p.font.size = Pt(12)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(8)

    p = tf6_r.add_paragraph()
    p.text = "• Dual-Ring WAL & Direct I/O: Kernel write buffers are bypassed via direct I/O barriers. The 2-ring WAL guarantees non-interfering commit pipelining."
    p.font.size = Pt(12)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(8)

    p = tf6_r.add_paragraph()
    p.text = "• Typestate Separation: Crate owns the state transition algebra (vrr::lifecycle); the host owns the physical I/O writes."
    p.font.size = Pt(12)
    p.font.color.rgb = C_TEXT_DARK

    s6.notes_slide.notes_text_frame.text = (
        "SOURCE: docs/uvrr-boot-gate.md §5, docs/uvrr-termination-obligations.md §4.\n"
        "REFERENCES: Pillai et al. (OSDI 2014), Chidambaram et al. (SOSP 2013), github.com/tigerbeetle/tigerbeetle superblock implementation.\n"
        "PRIMARY MESSAGE: Single-file flags are unsafe on modern storage. uVRR borrows TigerBeetle's 4-copy superblock quorum read/write "
        "model with direct I/O and dual-ring WAL to withstand torn sectors, misdirected writes, and bit rot."
    )

    # =========================================================================
    # SLIDE 7: The Equivalence Resolution: Gossip ≡ Recovery Minus Memory
    # =========================================================================
    s7 = prs.slides.add_slide(blank_layout)
    add_header(s7, "Consensus Equivalence", "Reconciling the Split Mind: VRR-2012 Recovery vs uVRR Gossip")
    add_footer(s7, 7, 9, "research/trex-vrr-gossip-equivalence.md §1-§3")

    # Banner callout with the formal theorem
    b7 = create_card(s7, Inches(0.8), Inches(1.45), Inches(11.733), Inches(0.85), bg_color=C_BLUE_LIGHT, border_color=C_BLUE)
    tb7_b = s7.shapes.add_textbox(Inches(1.0), Inches(1.5), Inches(11.333), Inches(0.75))
    tf7_b = tb7_b.text_frame
    tf7_b.word_wrap = True
    p7_b = tf7_b.paragraphs[0]
    p7_b.text = "The Equivalence Theorem: gossip ≡ recovery, with the memory precondition removed.\nVRR-2012 §4.3 and uVRR Trex gossip are the same catch-up mechanism at the liveness and network layers."
    p7_b.font.size = Pt(13)
    p7_b.font.bold = True
    p7_b.font.color.rgb = C_BLUE

    # Table comparing the dimensions
    t7_shape = s7.shapes.add_table(5, 3, Inches(0.8), Inches(2.55), Inches(11.733), Inches(4.2))
    table7 = t7_shape.table
    table7.columns[0].width = Inches(2.6)
    table7.columns[1].width = Inches(4.566)
    table7.columns[2].width = Inches(4.566)

    h7 = ["MECHANISM DIMENSION", "VRR-2012 §4.3 RECOVERY", "uVRR TREX GOSSIP"]
    for i, h in enumerate(h7):
        cell = table7.cell(0, i)
        cell.fill.solid()
        cell.fill.fore_color.rgb = C_NAVY_DARK
        p = cell.text_frame.paragraphs[0]
        p.text = h
        p.font.size = Pt(12)
        p.font.bold = True
        p.font.color.rgb = RGBColor(255, 255, 255)

    data7 = [
        ("Content Transferred", "Committed-and-accepted log suffix above the stale node's frontier.", "Identical: exactly the committed suffix above the stale frontier."),
        ("Authority Shape", "Primary's answer within a quorum of responses establishes installation evidence.", "Identical: current view's leader streams, earned via ordinary Phase 1."),
        ("Implicit Promise (Telescoping)", "Explicit recovery round establishes promise floor for subsequent normal traffic.", "Phase-2 at view v raises promise floor to v as if Prepare arrived (H1–H5 telescoping)."),
        ("Network Accounting", "Pull: 1 quorum read + 1 log transfer. Requires node to know its identity.", "Push: resend loop + streaming keep-up. Node requires zero prior knowledge.")
    ]

    for row_idx, row_data in enumerate(data7, start=1):
        for col_idx, text in enumerate(row_data):
            cell = table7.cell(row_idx, col_idx)
            cell.fill.solid()
            cell.fill.fore_color.rgb = RGBColor(255, 255, 255) if row_idx % 2 == 1 else RGBColor(241, 245, 249)
            p = cell.text_frame.paragraphs[0]
            p.text = text
            p.font.size = Pt(11)
            p.font.color.rgb = C_TEXT_DARK
            if col_idx == 0:
                p.font.bold = True

    s7.notes_slide.notes_text_frame.text = (
        "SOURCE: research/trex-vrr-gossip-equivalence.md §1-§3, formal/uvrr-lean/UVRR/Witness.lean.\n"
        "EQUIVALENCE PROOF: Theorems H1-H5 in UVRR/Witness.lean establish that the adopted era never exceeds committed era, "
        "and that the streamed prefix reconstructs the leader's committed history.\n"
        "PRIMARY MESSAGE: The perceived confusion between VRR-2012 recovery and Trex gossip is fully resolved. "
        "They are structurally identical in content, authority, and safety telescoping. The only difference is network economics: "
        "pull-once with remembered identity vs push-until-current without memory."
    )

    # =========================================================================
    # SLIDE 8: The Divergence Point: Amnesia vs Crash-Stop
    # =========================================================================
    s8 = prs.slides.add_slide(blank_layout)
    add_header(s8, "Safety Divergence", "Where Equivalence Parts: VRR-2012 Requires Memory; uVRR is Invariant")
    add_footer(s8, 8, 9, "research/trex-vrr-gossip-equivalence.md §3-§4")

    # Left: Classic VRR-2012 Vulnerability
    create_card(s8, Inches(0.8), Inches(1.45), Inches(5.7), Inches(5.3), bg_color=C_CARD_BG, border_color=C_CARD_BORDER)
    acc8_l = s8.shapes.add_shape(MSO_SHAPE.RECTANGLE, Inches(0.8), Inches(1.45), Inches(5.7), Inches(0.08))
    acc8_l.fill.solid()
    acc8_l.fill.fore_color.rgb = C_RED
    acc8_l.line.fill.background()

    tb8_l = s8.shapes.add_textbox(Inches(1.05), Inches(1.65), Inches(5.2), Inches(4.9))
    tf8_l = tb8_l.text_frame
    tf8_l.word_wrap = True

    p = tf8_l.paragraphs[0]
    p.text = "VRR-2012: The Amnesia Trap"
    p.font.size = Pt(16)
    p.font.bold = True
    p.font.color.rgb = C_RED
    p.space_after = Pt(10)

    p = tf8_l.add_paragraph()
    p.text = "• Trusting Remembered State: VRR-2012 §4.3 assumes the recovering node remembers its identity, its nonce, and its prior promises."
    p.font.size = Pt(12)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(8)

    p = tf8_l.add_paragraph()
    p.text = "• Michael et al. Counterexample: In the diskless setting, a replica can promise in view v, crash, return in view v-1 having forgotten the promise, and vote again. Linearizability is completely broken."
    p.font.size = Pt(12)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(8)

    p = tf8_l.add_paragraph()
    p.text = "• Fragile Protocol Requirement: Classic recovery is safe ONLY under stable storage across crashes. On a diskless normal path, this trust collapses."
    p.font.size = Pt(12)
    p.font.color.rgb = C_TEXT_DARK

    # Right: uVRR Gossip Posture
    create_card(s8, Inches(6.833), Inches(1.45), Inches(5.7), Inches(5.3), bg_color=C_CARD_BG, border_color=C_CARD_BORDER)
    acc8_r = s8.shapes.add_shape(MSO_SHAPE.RECTANGLE, Inches(6.833), Inches(1.45), Inches(5.7), Inches(0.08))
    acc8_r.fill.solid()
    acc8_r.fill.fore_color.rgb = C_GREEN
    acc8_r.line.fill.background()

    tb8_r = s8.shapes.add_textbox(Inches(7.083), Inches(1.65), Inches(5.2), Inches(4.9))
    tf8_r = tb8_r.text_frame
    tf8_r.word_wrap = True

    p = tf8_r.paragraphs[0]
    p.text = "uVRR: Zero Memory Assumption"
    p.font.size = Pt(16)
    p.font.bold = True
    p.font.color.rgb = C_GREEN
    p.space_after = Pt(10)

    p = tf8_r.add_paragraph()
    p.text = "• Zero Trust Across Crash: uVRR gossip trusts nothing the node remembers. A crashed node returns completely blank at era 0."
    p.font.size = Pt(12)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(8)

    p = tf8_r.add_paragraph()
    p.text = "• Invariant Under Amnesia: Authority comes strictly from the cluster's committed configuration, never from personal memory. A newborn has made no prior promises."
    p.font.size = Pt(12)
    p.font.color.rgb = C_TEXT_DARK
    p.space_after = Pt(8)

    p = tf8_r.add_paragraph()
    p.text = "• Boot Gate Synthesis: Clean start takes cheap in-cluster sync because memory was verified by FLUSHED quorum. Dirty start takes gossip because memory is absent."
    p.font.size = Pt(12)
    p.font.color.rgb = C_TEXT_DARK

    s8.notes_slide.notes_text_frame.text = (
        "SOURCE: research/trex-vrr-gossip-equivalence.md §3-§4, Michael et al. UW-CSE-16-08-02.\n"
        "PRIMARY MESSAGE: The split mind resolved: it was never two competing algorithms, but one catch-up mechanism with two safety postures. "
        "VRR-2012 recovery requires memory; uVRR gossip requires none. The boot gate is the durable classifier that selects between them at runtime."
    )

    # =========================================================================
    # SLIDE 9: System Synthesis: The Complete Verification Matrix
    # =========================================================================
    s9 = prs.slides.add_slide(blank_layout)
    add_header(s9, "System Architecture", "Formal Separation of Concerns: Storage, Core, and Network")
    add_footer(s9, 9, 9, "docs/uvrr-boot-gate.md §6, formal/uvrr-lean/UVRR/Witness.lean")

    # 4 Architecture Quadrants / Cards
    qw = Inches(5.7)
    qh = Inches(2.5)

    # Q1: Host Storage
    create_card(s9, Inches(0.8), Inches(1.45), qw, qh, bg_color=C_CARD_BG, border_color=C_CARD_BORDER)
    tb9_1 = s9.shapes.add_textbox(Inches(1.0), Inches(1.55), Inches(5.3), Inches(2.3))
    tf9_1 = tb9_1.text_frame
    tf9_1.word_wrap = True
    p = tf9_1.paragraphs[0]
    p.text = "1. Host Durable Boundary (LifecycleStore)"
    p.font.size = Pt(14)
    p.font.bold = True
    p.font.color.rgb = C_NAVY_DARK
    p.space_after = Pt(6)
    p = tf9_1.add_paragraph()
    p.text = "• Owns 4-copy superblock quorum I/O, dual-ring WAL, direct disk barriers.\n• Trait methods: read_copies() -> quorum read, commit() -> 4x forced write, drain() -> host WAL/grid flush."
    p.font.size = Pt(11)
    p.font.color.rgb = C_TEXT_DARK

    # Q2: Boot Gate Typestate Driver
    create_card(s9, Inches(6.833), Inches(1.45), qw, qh, bg_color=C_CARD_BG, border_color=C_CARD_BORDER)
    tb9_2 = s9.shapes.add_textbox(Inches(7.033), Inches(1.55), Inches(5.3), Inches(2.3))
    tf9_2 = tb9_2.text_frame
    tf9_2.word_wrap = True
    p = tf9_2.paragraphs[0]
    p.text = "2. Boot Gate Driver (vrr::lifecycle)"
    p.font.size = Pt(14)
    p.font.bold = True
    p.font.color.rgb = C_BLUE
    p.space_after = Pt(6)
    p = tf9_2.add_paragraph()
    p.text = "• Enforces write schedules by typestate: First, Clean, Crashed, Running, Halting, Draining, Halted.\n• Mints unforgeable tokens: Vouched (enables resume), Bumped (enables reincarnate), Rejoined (enables deferred latch)."
    p.font.size = Pt(11)
    p.font.color.rgb = C_TEXT_DARK

    # Q3: Sans-I/O Consensus Core
    create_card(s9, Inches(0.8), Inches(4.15), qw, qh, bg_color=C_CARD_BG, border_color=C_CARD_BORDER)
    tb9_3 = s9.shapes.add_textbox(Inches(1.0), Inches(4.25), Inches(5.3), Inches(2.3))
    tf9_3 = tb9_3.text_frame
    tf9_3.word_wrap = True
    p = tf9_3.paragraphs[0]
    p.text = "3. Sans-I/O Consensus Core (vrr::replica)"
    p.font.size = Pt(14)
    p.font.bold = True
    p.font.color.rgb = C_GREEN
    p.space_after = Pt(6)
    p = tf9_3.add_paragraph()
    p.text = "• Pure total transition function: tick + message + state -> state + list(messages).\n• Eliminates amnesia by construction: zero resume without Vouched token; era-0 input dropped by construction."
    p.font.size = Pt(11)
    p.font.color.rgb = C_TEXT_DARK

    # Q4: Rejoin Gossip & Witness Stream
    create_card(s9, Inches(6.833), Inches(4.15), qw, qh, bg_color=C_CARD_BG, border_color=C_CARD_BORDER)
    tb9_4 = s9.shapes.add_textbox(Inches(7.033), Inches(4.25), Inches(5.3), Inches(2.3))
    tf9_4 = tb9_4.text_frame
    tf9_4.word_wrap = True
    p = tf9_4.paragraphs[0]
    p.text = "4. Outer Rejoin Gossip & Witnesses"
    p.font.size = Pt(14)
    p.font.bold = True
    p.font.color.rgb = C_PURPLE
    p.space_after = Pt(6)
    p = tf9_4.add_paragraph()
    p.text = "• Outer protocol: joiner gossips to all nodes; leader streams Phase 2s + commits.\n• Lean verified (UVRR/Witness.lean): W-era-bound (adopt ≤ committed), W-era-exact (adopt = leader era), W-replay."
    p.font.size = Pt(11)
    p.font.color.rgb = C_TEXT_DARK

    s9.notes_slide.notes_text_frame.text = (
        "SOURCE: docs/uvrr-boot-gate.md §6, formal/uvrr-lean/UVRR/Witness.lean, src/lifecycle.rs.\n"
        "SYNTHESIS: Complete architectural picture. The host owns durable I/O; the boot gate enforces write schedules through typestates; "
        "the sans-I/O core runs consensus without amnesia; and the outer gossip protocol brings uninitialised nodes to the current era "
        "before they touch the consensus engine."
    )

    # Save
    prs.save(output_pptx_path)
    print(f"Presentation successfully saved to {output_pptx_path}")

if __name__ == "__main__":
    out_path = sys.argv[1] if len(sys.argv) > 1 else "docs/slides/uvrr-lifecycle-and-equivalence.pptx"
    build_deck(out_path)
