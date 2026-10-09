#!/usr/bin/python3.12
"""Dump the AT-SPI tree of the PVC window (external client; system python3 + pyatspi).
usage: atspi_dump.py [--app NAME] [--out FILE] [--wait SECONDS]
Prints/writes JSON: [{depth, role, name, states, text, value, attrs, interfaces, extents, actions}]"""
import sys, json, time, argparse
import pyatspi

ap = argparse.ArgumentParser()
ap.add_argument("--app", default="pvc-native")
ap.add_argument("--out")
ap.add_argument("--brief", action="store_true", help="also print one line per node")
ap.add_argument("--wait", type=float, default=10.0)
a = ap.parse_args()

def find_app():
    desk = pyatspi.Registry.getDesktop(0)
    names = []
    for i in range(desk.childCount):
        try:
            c = desk.getChildAtIndex(i)
        except Exception:
            continue
        if c is None:
            continue
        names.append(c.name)
        if a.app in (c.name or ""):
            return c, names
    return None, names

def node(o, depth):
    d = {"depth": depth, "role": o.getRoleName(), "name": o.name, "description": o.description}
    d["states"] = sorted(s.value_name.replace("ATSPI_STATE_", "").replace("STATE_", "").lower() for s in o.getState().getStates())
    ifs = [i.replace("org.a11y.atspi.", "") for i in o.get_interfaces()] if hasattr(o, "get_interfaces") else []
    d["interfaces"] = ifs
    try:
        d["attrs"] = dict(o.getAttributes() and (kv.split(":", 1) for kv in o.getAttributes()))
    except Exception as e:
        d["attrs"] = {}
    try:
        if "Text" in ifs:
            t = o.queryText()
            d["text"] = t.getText(0, -1)
            d["caret"] = t.caretOffset
    except Exception as e:
        d["text_err"] = str(e)
    try:
        if "Value" in ifs:
            d["value"] = o.queryValue().currentValue
    except Exception:
        pass
    try:
        if "Component" in ifs:
            e = o.queryComponent().getExtents(pyatspi.DESKTOP_COORDS)
            d["extents"] = [e.x, e.y, e.width, e.height]
    except Exception:
        pass
    try:
        if "Action" in ifs:
            ac = o.queryAction()
            d["actions"] = [ac.getName(i) for i in range(ac.nActions)]
    except Exception:
        pass
    try:
        d["relations"] = [(r.getRelationType().value_name.replace("RELATION_", "").lower(), [t.name for t in (r.getTarget(i) for i in range(r.getNTargets()))]) for r in o.getRelationSet()]
    except Exception:
        pass
    return d

out = []
def walk(o, depth):
    out.append(node(o, depth))
    for i in range(o.childCount):
        try:
            c = o.getChildAtIndex(i)
        except Exception:
            continue
        if c is not None:
            walk(c, depth + 1)

t0 = time.time(); app = None; names = []
while time.time() - t0 < a.wait:
    app, names = find_app()
    if app is not None:
        break
    time.sleep(0.3)
if app is None:
    print(json.dumps({"error": "app not found", "apps_on_bus": names}), file=sys.stderr)
    sys.exit(2)
walk(app, 0)
s = json.dumps(out, indent=1, ensure_ascii=False)
if a.out:
    open(a.out, "w").write(s)
if a.brief:
    for d in out:
        extra = ("  text=%r" % d["text"]) if "text" in d else ""
        extra += ("  attrs=%s" % d["attrs"]) if d.get("attrs") else ""
        st = [x for x in d["states"] if not x.startswith("atspi_") or True]
        print("%s%-14s %-28r [%s]%s" % ("  " * d["depth"], d["role"], d["name"], ",".join(st), extra))
else:
    print(s)
