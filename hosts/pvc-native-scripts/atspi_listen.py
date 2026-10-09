#!/usr/bin/python3.12
"""Log AT-SPI events (object:announcement = AT-SPI's live-region carrier in accesskit_atspi_common,
object:state-changed, object:property-change:accessible-name, object:text-changed) as JSON lines.
usage: atspi_listen.py OUTFILE SECONDS"""
import sys, json, time
import pyatspi
from gi.repository import GLib
out = open(sys.argv[1], "w")
secs = float(sys.argv[2])
def cb(e):
    try:
        src = e.source
        rec = {"t": round(time.time(), 3), "type": e.type, "detail1": e.detail1, "detail2": e.detail2,
               "any_data": e.any_data if isinstance(e.any_data, (str, int, float, type(None))) else str(e.any_data),
               "source_role": src.getRoleName() if src else None, "source_name": src.name if src else None}
        out.write(json.dumps(rec, ensure_ascii=False) + "\n"); out.flush()
    except Exception as ex:
        out.write(json.dumps({"err": str(ex)}) + "\n"); out.flush()
for t in ("object:announcement", "object:state-changed", "object:property-change:accessible-name", "object:text-changed"):
    pyatspi.Registry.registerEventListener(cb, t)
GLib.timeout_add(int(secs * 1000), pyatspi.Registry.stop)
out.write(json.dumps({"listening": True, "t": round(time.time(), 3)}) + "\n"); out.flush()
pyatspi.Registry.start()
