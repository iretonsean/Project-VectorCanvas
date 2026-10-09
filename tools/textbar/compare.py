#!/usr/bin/env python3
"""A8 comparison: LDR-FLIP (flip-evaluator) PVC vs Chromium, Firefox vs Chromium, plus line-break equality.
usage: compare.py <work_dir> <out_dir>   (work_dir has chromium/ chromium-lcd/ firefox/ pvc/)"""
import json, sys, os, itertools
import numpy as np
from PIL import Image
import flip_evaluator as flip

work, out = sys.argv[1], sys.argv[2]
SIZES = [11, 13, 16, 24]; SCALES = [1, 2]; SCRIPTS = ['latin', 'zh-CN']

def load(engine, name):
    im = Image.open(f'{work}/{engine}/{name}.png').convert('RGB')
    return np.asarray(im, dtype=np.float32) / 255.0

def flip_mean(ref, test):
    assert ref.shape == test.shape, (ref.shape, test.shape)
    e, m, p = flip.evaluate(np.ascontiguousarray(ref), np.ascontiguousarray(test), 'LDR', inputsRGB=True, applyMagma=False, computeMeanError=True)
    mask = ((ref < 0.9999).any(axis=2)) | ((test < 0.9999).any(axis=2))
    ink = float(e[..., 0][mask].mean()) if mask.any() else 0.0
    return float(m), ink, float(p['ppd']), int(mask.sum())

def shift(img, dx, dy):
    out_ = np.ones_like(img)
    h, w = img.shape[:2]
    ys, yd = (slice(0, h - dy), slice(dy, h)) if dy >= 0 else (slice(-dy, h), slice(0, h + dy))
    xs, xd = (slice(0, w - dx), slice(dx, w)) if dx >= 0 else (slice(-dx, w), slice(0, w + dx))
    out_[yd, xd] = img[ys, xs]
    return out_

def aligned(ref, test, S):
    best = None
    for dx, dy in itertools.product(range(-2 * S, 2 * S + 1), repeat=2):
        t = shift(test, dx, dy)
        ssd = float(((ref - t) ** 2).sum())
        if best is None or ssd < best[0]: best = (ssd, dx, dy)
    _, dx, dy = best
    m, ink, _, _ = flip_mean(ref, shift(test, dx, dy))
    return m, ink, dx, dy

def lcd_px(img):
    return int(((np.abs(img[..., 0] - img[..., 1]) > 2/255) | (np.abs(img[..., 1] - img[..., 2]) > 2/255)).sum())

B = {e: json.load(open(f'{work}/{e}/breaks_{e}.json')) for e in ['chromium', 'chromium-lcd', 'firefox', 'pvc']}
HAVE_NOHINT = os.path.exists(f'{work}/chromium-nohint/breaks_chromium-nohint.json')
if HAVE_NOHINT: B['chromium-nohint'] = json.load(open(f'{work}/chromium-nohint/breaks_chromium-nohint.json'))
meta = {e: {'version': B[e].get('version'), 'backend': B[e].get('backend')} for e in B}
res = {'meta': meta, 'ppd_note': 'flip-evaluator 1.7 default viewing conditions', 'cells': {}}
for script in SCRIPTS:
    for size in SIZES:
        for S in SCALES:
            name = f'{script}_s{size}_x{S}'
            ref = load('chromium', name); ref_lcd = load('chromium-lcd', name)
            ff = load('firefox', name); pv = load('pvc', name)
            c = {'lcd_px': {'chromium': lcd_px(ref), 'firefox': lcd_px(ff), 'pvc': lcd_px(pv), 'chromium-lcd': lcd_px(ref_lcd)}}
            m1, k1, ppd, nink = flip_mean(ref, pv)
            m2, k2, _, _ = flip_mean(ref, ff)
            a1 = aligned(ref, pv, S); a2 = aligned(ref, ff, S)
            m1l, k1l, _, _ = flip_mean(ref_lcd, pv)
            m2l, k2l, _, _ = flip_mean(ref_lcd, ff)
            c.update({'ppd': ppd, 'ink_pixels': nink,
                      'pvc_vs_chromium': {'flip_mean': m1, 'flip_ink': k1, 'aligned_flip_mean': a1[0], 'aligned_flip_ink': a1[1], 'shift': [a1[2], a1[3]]},
                      'firefox_vs_chromium': {'flip_mean': m2, 'flip_ink': k2, 'aligned_flip_mean': a2[0], 'aligned_flip_ink': a2[1], 'shift': [a2[2], a2[3]]},
                      'pvc_vs_chromium_lcd': {'flip_mean': m1l, 'flip_ink': k1l},
                      'firefox_vs_chromium_lcd': {'flip_mean': m2l, 'flip_ink': k2l},
                      'pass_primary': m1 <= m2, 'pass_ink': k1 <= k2, 'pass_aligned': a1[0] <= a2[0],
                      'pass_lcd_secondary': m1l <= m2l,
                      'ratio_pvc_over_firefox': (m1 / m2) if m2 > 0 else None})
            # line breaks
            bc = B['chromium']['cells'][name]['breaks']; bp = B['pvc']['cells'][name]['breaks']; bf = B['firefox']['cells'][name]['breaks']
            mism = []; mism_ff = []; n = 0
            for item_id, v in bc.items():
                if item_id not in bp: continue
                # only wrapped (paragraph) items are line-break tests; labels are single line
                n += 1
                if v['starts'] != bp[item_id]['starts']:
                    mism.append({'id': item_id, 'chromium': v['starts'], 'pvc': bp[item_id]['starts']})
                if v['starts'] != bf[item_id]['starts']:
                    mism_ff.append({'id': item_id, 'chromium': v['starts'], 'firefox': bf[item_id]['starts']})
            if HAVE_NOHINT:
                rn = load('chromium-nohint', name)
                n1, nk1, _, _ = flip_mean(rn, pv); n2, nk2, _, _ = flip_mean(rn, ff)
                bn = B['chromium-nohint']['cells'][name]['breaks']
                mp = [i for i, v in bn.items() if i in bp and v['starts'] != bp[i]['starts']]
                mf = [i for i, v in bn.items() if i in bf and v['starts'] != bf[i]['starts']]
                c['nohint_secondary'] = {'pvc_flip_mean': n1, 'firefox_flip_mean': n2, 'pass': n1 <= n2, 'pvc_linebreak_mismatch_items': mp, 'firefox_linebreak_mismatch_items': mf}
            c['linebreak'] = {'items_compared': n, 'pvc_mismatches': mism, 'firefox_mismatches': mism_ff, 'pass': not mism}
            res['cells'][name] = c
            print(name, f"pvc {m1:.5f} ff {m2:.5f} pass={m1<=m2} lb_mismatch={len(mism)}", flush=True)
os.makedirs(out, exist_ok=True)
json.dump(res, open(f'{out}/results.json', 'w'), indent=1, ensure_ascii=False)
