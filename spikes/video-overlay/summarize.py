#!/usr/bin/env python3
"""Print a per-phase table from target/logs/*.json (numbers only come from the measured samples)."""
import glob, json, os, sys
rows = []
for f in sorted(glob.glob(os.path.join(os.path.dirname(os.path.abspath(__file__)), "target/logs/*.json"))):
    d = json.load(open(f))
    s = {x["t"]: x for x in d["samples"]}
    end_t = d["toggles_end_t"]
    last = d["samples"][-1]
    def span(a, b):
        if a not in s or b not in s:
            return (0, 0)
        return (s[b]["frame_drop"] - s[a]["frame_drop"], s[b]["frames"] - s[a]["frames"])
    pre = span(2, 5); tog = span(5, end_t); post = span(end_t, last["t"])
    def pct(x): return f"{100*x[0]/x[1]:.2f}% ({x[0]}/{x[1]})" if x[1] else "n/a"
    startup = (s[1]["frame_drop"], s[1]["frames"]) if 1 in s else (0, 0)
    cpu = lambda a, b: sum(s[t]["cpu_pct"] for t in range(a, b + 1) if t in s) / max(1, len([t for t in range(a, b + 1) if t in s]))
    rows.append((os.path.basename(f)[:-5], d["gdk_backend"], d["hwdec_current"],
                 f'{d["drop_pct_vo"]:.2f}% ({d["frame_drop"]}/{d["frames"]})', pct((startup[0], startup[1])), pct(pre), pct(tog), pct(post),
                 f"{cpu(2,5):.1f}", f"{cpu(end_t+1,last['t']):.1f}", d["rss_kb_t5"], d["rss_kb_end"], d["fullscreen_toggles_done"], d["gl_errors"]))
hdr = ("run","backend","hwdec","total drops","t<=1s","t2-5 (steady)","toggles","post-toggles","cpu% t2-5","cpu% post","rss t5 kB","rss end kB","toggles","GL err")
print(" | ".join(hdr))
for r in rows: print(" | ".join(str(x) for x in r))
