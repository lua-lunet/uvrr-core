#!/usr/bin/env python3
"""Finite checks for the arbitrary-size weighted reachability and phantom proofs.

The checker is standard-library-only. --render additionally needs matplotlib.
Identity 0 is a live positive-weight leader. Live sets are enumerated up to
permutation preserving identity 0; labelled counts restore those permutations.
An abstract quorum witness is never counted as a received message.
"""
from __future__ import annotations

import argparse
import csv
import html
import itertools
import json
import math
from functools import lru_cache
from pathlib import Path

OUT = Path(__file__).resolve().parent / 'generated'


def weight(w, mask):
    return sum(x for i, x in enumerate(w) if mask >> i & 1)


def majority(w, mask):
    return 2 * weight(w, mask) > sum(w)


def margin(w, live):
    return 2 * weight(w, live) - sum(w)


def positive_leader(w, live):
    return bool(live & 1) and w[0] > 0


def minority_first_path(source, target, live):
    """An L1-shortest path: helpful edits, then target-bounded edits."""
    assert len(source) == len(target)
    assert margin(source, live) > 0 and margin(target, live) > 0
    assert positive_leader(source, live) and positive_leader(target, live)
    current = list(source)
    steps = [('initial', tuple(current))]
    for is_live, direction in [(False, -1), (True, 1), (True, -1), (False, 1)]:
        for i in range(len(current)):
            if bool(live >> i & 1) != is_live:
                continue
            while (target[i] - current[i]) * direction > 0:
                current[i] += direction
                steps.append((f'{"increment" if direction > 0 else "decrement"} n{i}', tuple(current)))
    assert tuple(current) == tuple(target)
    return steps


def minimal_live_quorum(w, live):
    """Exact subset-sum DP, keeping an actual responding identity witness."""
    sums = {w[0]: 1}
    for i in range(1, len(w)):
        if live >> i & 1:
            added = {total + w[i]: mask | (1 << i) for total, mask in sums.items()}
            for total, mask in added.items():
                sums.setdefault(total, mask)
    p = min(total for total in sums if 2 * total > sum(w))
    return p, sums[p]


def phantom_completion(w, live):
    """At most three added weight units / two fresh phantom identities."""
    p, promises = minimal_live_quorum(w, live)
    delta = max(0, 2 * p - 2 * w[0] + 1 - sum(w))
    assert delta <= 3
    steps = [('initial', tuple(w))]
    remaining = delta
    current = list(w)
    while remaining:
        current.append(0)  # JOIN introduces no vote and requires no response.
        steps.append((f'join phantom n{len(current)-1}', tuple(current)))
        for _ in range(min(2, remaining)):
            current[-1] += 1
            remaining -= 1
            steps.append((f'increment phantom n{len(current)-1}', tuple(current)))
    completed = tuple(current)
    full = (1 << len(completed)) - 1
    abstract = (full ^ promises) | 1
    assert promises & abstract == 1
    assert promises & ~live == 0
    assert majority(completed, promises) and majority(completed, abstract)
    assert len(completed) - len(w) <= 2
    # No smaller padding can suffice for ANY live leader-containing quorum:
    # p is the minimum such quorum weight; complement is its largest mate.
    for less in range(delta):
        total = sum(w) + less
        assert 2 * (total - p + w[0]) <= total
    return delta, promises, abstract, steps


@lru_cache(maxsize=None)
def exhaustive_cross_intersection(left, right):
    """Independent powerset check; complements maximise a disjoint quorum."""
    n = max(len(left), len(right))
    left = left + (0,) * (n - len(left))
    right = right + (0,) * (n - len(right))
    full = (1 << n) - 1
    for mask in range(full + 1):
        if majority(left, mask):
            assert not majority(right, full ^ mask), (left, right, mask)
        if majority(right, mask):
            assert not majority(left, full ^ mask), (right, left, mask)
    return True


def check_path(steps, live, exhaustive=False):
    for _, w in steps:
        assert all(0 <= x <= 2 for x in w)
        assert positive_leader(w, live)
        assert margin(w, live) > 0
    for (_, left), (_, right) in itertools.pairwise(steps):
        n = max(len(left), len(right))
        a = left + (0,) * (n - len(left))
        b = right + (0,) * (n - len(right))
        assert sum(abs(x - y) for x, y in zip(a, b)) <= 1
        if exhaustive:
            exhaustive_cross_intersection(left, right)


def rows_for(n, live_count):
    live = (1 << live_count) - 1
    for leader in (1, 2):
        for rest in itertools.product(range(3), repeat=n - 1):
            w = (leader,) + rest
            if margin(w, live) > 0:
                yield w


def audit_states(max_n):
    result = []
    padding_histogram = [0] * 4
    for n in range(1, max_n + 1):
        checked = labelled = largest_pad = largest_path = 0
        for live_count in range(1, n + 1):
            live = (1 << live_count) - 1
            multiplicity = math.comb(n - 1, live_count - 1)
            hub = tuple(2 if live >> i & 1 else 0 for i in range(n))
            for w in rows_for(n, live_count):
                delta, _, _, padded = phantom_completion(w, live)
                check_path(padded, live, exhaustive=n <= 4)
                hub_path = minority_first_path(w, hub, live)
                check_path(hub_path, live, exhaustive=n <= 4)
                canonical_delta = weight(w, live) - (sum(w) - weight(w, live)) - 1
                assert canonical_delta >= 0
                canonical_total = sum(w) + canonical_delta
                a = weight(w, live)
                d = sum(w) - a
                assert canonical_total == 2 * a - 1
                assert 2 * a > canonical_total
                assert 2 * (d + canonical_delta + w[0]) > canonical_total
                checked += 1
                labelled += multiplicity
                padding_histogram[delta] += multiplicity
                largest_pad = max(largest_pad, delta)
                largest_path = max(largest_path, len(hub_path) - 1)
        result.append(dict(identities=n, representative_profiles=checked,
                           labelled_profiles=labelled, failures=0,
                           maximum_phantom_weight=largest_pad,
                           maximum_hub_path=largest_path))
        print(f'n={n}: {checked:,} representatives / {labelled:,} labelled profiles, no failures', flush=True)
    return result, padding_histogram


def audit_pairs(max_n):
    result = []
    for n in range(1, max_n + 1):
        checked = labelled = longest = 0
        for live_count in range(1, n + 1):
            live = (1 << live_count) - 1
            profiles = list(rows_for(n, live_count))
            multiplicity = math.comb(n - 1, live_count - 1)
            for source in profiles:
                for target in profiles:
                    steps = minority_first_path(source, target, live)
                    check_path(steps, live, exhaustive=n <= 3)
                    distance = sum(abs(x - y) for x, y in zip(source, target))
                    assert len(steps) - 1 == distance
                    checked += 1
                    labelled += multiplicity
                    longest = max(longest, distance)
        result.append(dict(identities=n, ordered_representative_pairs=checked,
                           ordered_labelled_pairs=labelled, maximum_path=longest, failures=0))
        print(f'pairs n={n}: {checked:,} representatives, no failures', flush=True)
    return result


def control_checks():
    # Missing initial live majority is a real violated premise, not a search miss.
    assert margin((1, 1, 1), 1) < 0
    # A jump of two can destroy universal cross-era intersection.
    try:
        exhaustive_cross_intersection((1, 1, 1, 0, 0), (1, 1, 1, 1, 1))
    except AssertionError:
        pass
    else:
        raise AssertionError('unsafe two-vote jump was not rejected')
    # Too much phantom weight consumes the responding quorum.
    assert not majority((1, 1, 1, 2), 0b011)
    # Sharp three-unit padding requirement with a fixed weight-one leader.
    delta, promises, abstract, _ = phantom_completion((1, 2, 2, 1), 0b0111)
    assert delta == 3 and promises == 0b0111
    assert abstract & promises == 1
    # Abstract membership cannot be silently substituted for received messages.
    completed = (1, 2, 2, 1, 2, 1)
    assert majority(completed, abstract)
    assert not majority(completed, abstract & 0b0111)
    # Scale changes preserve every quorum, including tied even totals.
    scale_cases = 0
    for n in range(1, 8):
        for w in itertools.product((0, 1), repeat=n):
            doubled = tuple(2 * x for x in w)
            for mask in range(1 << n):
                assert majority(w, mask) == majority(doubled, mask)
            scale_cases += 1
    return dict(controls_passed=6, scale_profiles_checked=scale_cases)


def csv_write(name, rows):
    with (OUT / name).open('w', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


def named(mask, names):
    return ', '.join(name for i, name in enumerate(names) if mask >> i & 1) or '∅'


def schedule_rows(steps, live, names):
    final_width = len(steps[-1][1])
    result = []
    for index, (op, w) in enumerate(steps):
        padded = w + (0,) * (final_width - len(w))
        p, prom = minimal_live_quorum(w, live)
        total = sum(w)
        result.append(dict(stage=index, operation=op, **dict(zip(names, padded)),
                           total=total, majority=total // 2 + 1,
                           available_weight=weight(w, live), unavailable_weight=total-weight(w, live),
                           margin=margin(w, live), responding_quorum=named(prom, names)))
    return result


def examples():
    # Sharp fixed-leader example: at most two phantom identities are enough.
    live = 0b0111
    delta, promises, abstract, steps = phantom_completion((1, 2, 2, 1), live)
    names = ['Leader', 'A', 'B', 'Unavailable', 'Phantom X', 'Phantom Y']
    rows = schedule_rows(steps, live, names)
    csv_write('phantom_schedule.csv', rows)
    # Replacement of unavailable Old by live New, with leader retained.
    source = (1, 1, 1, 1, 1, 0)
    target = (1, 1, 1, 1, 0, 1)
    replacement_live = 0b101111
    direct = minority_first_path(source, target, replacement_live)
    check_path(direct, replacement_live, exhaustive=True)
    csv_write('replacement_schedule.csv', schedule_rows(direct, replacement_live,
              ['Leader', 'A', 'B', 'C', 'Old', 'New']))
    return dict(phantom_weight=delta, live_names=['Leader', 'A', 'B'],
                promise_quorum=named(promises, names),
                abstract_quorum=named(abstract, names),
                intersection='Leader', schedule=rows,
                replacement_steps=len(direct)-1,
                replacement_note='This is a safe/live reachability path, not a claim of simultaneous disjoint responding quorums.')


def render(summary):
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    plt.rcParams.update({'font.family': 'DejaVu Sans', 'font.size': 11,
                         'axes.spines.top': False, 'axes.spines.right': False,
                         'figure.facecolor': '#faf9f5', 'axes.facecolor': '#faf9f5'})
    colours = ['#167d9a', '#557a46', '#cb8b2b', '#ad4d46']
    coverage = summary['profiles']
    fig, ax = plt.subplots(figsize=(10, 5.5), constrained_layout=True)
    ns = [r['identities'] for r in coverage]
    counts = [r['labelled_profiles'] for r in coverage]
    ax.bar(ns, counts, color=colours[0])
    ax.set_yscale('log'); ax.set_xticks(ns)
    ax.set_xlabel('Existing identities (including unavailable and zero-weight identities)')
    ax.set_ylabel('Labelled weight / availability profiles checked')
    ax.set_title('Every enumerated live-majority profile admits phantom completion', loc='left', pad=20)
    for n, count in zip(ns, counts):
        ax.annotate(f'{count:,}', (n, count), xytext=(0, 5), textcoords='offset points', ha='center', fontsize=9)
    ax.text(.01, .97, '0 failures · positive live leader · weights in {0,1,2}', transform=ax.transAxes,
            va='top', fontsize=10, bbox=dict(facecolor='white', edgecolor='none', alpha=.9))
    fig.savefig(OUT/'coverage.png', dpi=180); fig.savefig(OUT/'coverage.svg'); plt.close(fig)

    rows = summary['example']['schedule']
    fig, axes = plt.subplots(2, 1, figsize=(11, 8), constrained_layout=True,
                             gridspec_kw={'height_ratios': [1, 1]})
    names = ['Leader', 'A', 'B', 'Unavailable', 'Phantom X', 'Phantom Y']
    data = [[r[name] for r in rows] for name in names]
    im = axes[0].imshow(data, vmin=0, vmax=2, cmap='YlGnBu', aspect='auto')
    axes[0].set_yticks(range(len(names)), names)
    axes[0].set_xticks(range(len(rows)), [str(r['stage']) for r in rows])
    axes[0].set_title('Two phantom identities suffice in the sharp three-unit example', loc='left', pad=16)
    axes[0].set_xlabel('Stage (joins introduce weight 0)')
    for y, row in enumerate(data):
        for x, value in enumerate(row):
            axes[0].text(x, y, str(value), ha='center', va='center', color='white' if value == 2 else '#152d3c')
    stages = [r['stage'] for r in rows]
    axes[1].plot(stages, [r['available_weight'] for r in rows], 'o-', lw=2.5,
                 label='Available weight (all responses possible)', color=colours[0])
    axes[1].plot(stages, [r['unavailable_weight'] for r in rows], 's-', lw=2.5,
                 label='Unavailable + phantom weight', color=colours[3])
    axes[1].plot(stages, [r['majority'] for r in rows], '--', lw=2,
                 label='Strict-majority threshold', color=colours[2])
    axes[1].set_xticks(stages); axes[1].set_xlabel('Stage'); axes[1].set_ylabel('Weight')
    axes[1].legend(loc='upper left', fontsize=9)
    axes[1].set_title('The responding side remains a majority at every stage', loc='left')
    fig.savefig(OUT/'phantom-construction.png', dpi=180); fig.savefig(OUT/'phantom-construction.svg'); plt.close(fig)

    fig, axes = plt.subplots(1, 2, figsize=(11, 4.8), constrained_layout=True)
    hist = summary['phantom_weight_histogram']
    axes[0].bar(range(4), hist, color=colours)
    axes[0].set_yscale('log'); axes[0].set_xticks(range(4)); axes[0].set_xlabel('Minimum added phantom weight')
    axes[0].set_ylabel('Labelled profiles'); axes[0].set_title('At most 3 units / 2 identities', loc='left')
    for x, count in enumerate(hist):
        axes[0].annotate(f'{count:,}', (x,count), xytext=(0,5),textcoords='offset points',ha='center',fontsize=9)
    pairs = summary['pairs']
    axes[1].plot([r['identities'] for r in pairs], [r['maximum_path'] for r in pairs], 'o-', color=colours[0], lw=2)
    axes[1].set_xticks([r['identities'] for r in pairs]); axes[1].set_xlabel('Identities in source ∪ target')
    axes[1].set_ylabel('Longest shortest unit-edit path checked')
    axes[1].set_title('Every admissible endpoint pair connected', loc='left')
    fig.savefig(OUT/'bounds.png', dpi=180); fig.savefig(OUT/'bounds.svg'); plt.close(fig)
    from matplotlib.patches import Circle
    fig, ax = plt.subplots(figsize=(11, 5.8), constrained_layout=True)
    ax.set_xlim(-3.8, 3.8); ax.set_ylim(-2.2, 2.5); ax.set_aspect('equal'); ax.axis('off')
    ax.add_patch(Circle((-1.1, 0), 1.75, facecolor='#ad4d4620', edgecolor=colours[3], lw=2))
    ax.add_patch(Circle((1.1, 0), 1.75, facecolor='#167d9a20', edgecolor=colours[0], lw=2))
    ax.text(-1.7, 2.05, 'Abstract companion quorum', ha='center', weight='bold', color=colours[3])
    ax.text(1.7, 2.05, 'Responding promise quorum', ha='center', weight='bold', color=colours[0])
    ax.text(-1.6, 0.15, 'Unavailable: 1\nPhantom X: 2\nPhantom Y: 1', ha='center', va='center', linespacing=1.8)
    ax.text(1.65, 0.15, 'Live A: 2\nLive B: 2', ha='center', va='center', linespacing=2)
    ax.text(0, 0, 'Leader\n1', ha='center', va='center', weight='bold', fontsize=16)
    ax.text(-1.65, -1.12, 'Weight 5', ha='center', weight='bold')
    ax.text(1.65, -1.12, 'Weight 5', ha='center', weight='bold')
    ax.text(0, -2.0, 'Total weight 9 · majority threshold 5 · intersection = {Leader}', ha='center', fontsize=13)
    ax.set_title('The phantom identities occur outside the promise recipients', loc='left', pad=15)
    fig.savefig(OUT/'quorum-witness.png', dpi=180); fig.savefig(OUT/'quorum-witness.svg'); plt.close(fig)
    render_html(summary)


def render_html(summary):
    def table(rows):
        keys = list(rows[0]); parts=['<table><thead><tr>']
        parts.extend(f'<th>{html.escape(k.replace("_", " "))}</th>' for k in keys)
        parts.append('</tr></thead><tbody>')
        for row in rows:
            parts.append('<tr>'+''.join(f'<td>{html.escape(str(row[k]))}</td>' for k in keys)+'</tr>')
        return ''.join(parts)+'</tbody></table>'
    example = summary['example']
    total = sum(r['labelled_profiles'] for r in summary['profiles'])
    pair_total = sum(r['ordered_labelled_pairs'] for r in summary['pairs'])
    doc = f'''<!doctype html><html lang="en"><meta charset="utf-8"><title>Weighted reconfiguration: reachability and phantom overlap</title>
<style>body{{max-width:1100px;margin:3rem auto;padding:0 2rem;background:#faf9f5;color:#19313d;font:17px/1.6 system-ui}}h1{{font-size:2.5rem;line-height:1.15}}h2{{margin-top:3rem}}.card{{background:#fff;padding:1.4rem 2rem;border-left:5px solid #167d9a;margin:1.5rem 0}}img{{width:100%;height:auto}}table{{border-collapse:collapse;font-size:13px;display:block;overflow:auto}}td,th{{border-bottom:1px solid #d7dcdb;text-align:left;padding:8px}}th{{background:#e7eeed}}code{{background:#e7eeed;padding:2px 5px}}a{{color:#167d9a}}</style>
<p>REPRODUCIBLE MATHEMATICAL EVIDENCE</p><h1>Membership arithmetic has room for phantom identities.</h1>
<div class="card"><b>General phantom construction.</b> If live weight A exceeds unavailable weight D and the leader is live with positive weight, add A−D−1 units of phantom weight. The total becomes 2A−1. The live set and the unavailable set plus the leader are both strict-majority quorums; their intersection is exactly the leader. All requested promises are on the live side.</div>
<p>Each added identity has weight 0, 1 or 2. Choosing a minimum-weight responding quorum improves the construction to at most <b>three phantom units, on at most two phantom identities</b>. The bound is sharp for a fixed leader of weight 1.</p>
<img src="quorum-witness.png" alt="Abstract companion and responding promise quorum intersect only at the leader"><img src="phantom-construction.png" alt="Phantom weights by stage and available majority">
<p><b>Final promise quorum:</b> {example['promise_quorum']}. <b>Abstract companion:</b> {example['abstract_quorum']}. <b>Intersection:</b> Leader. Phantom identities receive no counted promise requests.</p>
{table(example['schedule'])}
<h2>Any admissible source and target are connected</h2>
<p>Keep a stable set of responding identities and a positive live leader. Both endpoints must have a live strict weighted majority. Use the union of their identities, giving absent identities weight zero. First decrease unavailable weights and increase live weights towards the target. Then decrease live weights and increase unavailable weights to the target. Every row has a responding majority, every edge changes one weight by one, and every adjacent pair of majority families intersects.</p>
<p>The number of unit edits is exactly <code>Σ |sourceᵢ − targetᵢ|</code>, the shortest possible among single-unit-edit paths. A join at zero or leave at zero changes identity membership without changing quorum arithmetic. Doubling and integral halving preserve quorum families.</p>
<img src="coverage.png" alt="Exhaustive profile coverage"><p>{total:,} labelled profiles, represented by all availability counts and all labelled weight vectors through {summary['max_n']} existing identities: zero failures. Leader identity is fixed at index zero; arbitrary leader names follow by renaming. All finite sizes are covered by the mathematical proof, rather than extrapolated from this chart.</p>
{table(summary['profiles'])}
<img src="bounds.png" alt="Phantom bound and endpoint path length"><p>{pair_total:,} ordered labelled endpoint pairs through {summary['pair_max_n']} identities: zero failures, every constructed path shortest in unit edits.</p>
{table(summary['pairs'])}
<h2>Exactly what is established</h2>
<p>The phantom theorem constructs an <b>abstract companion quorum</b> and a <b>fully responding promise quorum</b>. The reachability theorem supplies a responding majority at every committed configuration. Neither theorem counts an unavailable identity as an acknowledgement or asserts that both disjoint arms simultaneously execute network rounds. That additional operational statement has different premises. A failed test of one fixed pivot or table is not a proof that no safe reconfiguration path exists.</p>
<p>Tests include negative controls for a missing live majority, an unsafe two-vote jump, excessive phantom padding, and substituting an abstract witness for actual received votes. Integer double/halve quorum equivalence is exhaustively checked separately.</p>
<p>Run <code>python check.py --render</code> from the parent directory. See <a href="../proof-analysis.md">the mathematical proof</a>, <a href="../semantics.md">definitions and source comparison</a>, and <a href="summary.json">machine-readable results</a>. Source context: <a href="https://github.com/DaveCTurner/paxos-membership/blob/raft-like-reconfiguration/paxos-reconf.tex">Turner’s LaTeX</a>. The general completion and reachability constructions here are stated and checked independently.</p></html>'''
    (OUT/'report.html').write_text(doc)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--max-n',type=int,default=8)
    parser.add_argument('--pair-max-n',type=int,default=5)
    parser.add_argument('--render',action='store_true')
    args=parser.parse_args()
    OUT.mkdir(parents=True,exist_ok=True)
    controls=control_checks()
    profiles,hist=audit_states(args.max_n)
    pairs=audit_pairs(args.pair_max_n)
    summary=dict(max_n=args.max_n,pair_max_n=args.pair_max_n,profiles=profiles,pairs=pairs,
                 phantom_weight_histogram=hist, controls=controls,
                 independent_cross_checks=exhaustive_cross_intersection.cache_info().currsize,
                 example=examples())
    csv_write('profile_coverage.csv',profiles); csv_write('pair_coverage.csv',pairs)
    (OUT/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
    if args.render:
        render(summary)
    print('All checks passed. Results:',OUT,flush=True)


if __name__=='__main__':
    main()
