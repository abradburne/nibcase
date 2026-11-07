#!/usr/bin/env python3
"""FinalBurn Neo's arcade DAT (its own ClrMamePro XML) as DSlot's [games] list: one set a line,
tab-separated: set, title, model (empty), year, maker. BIOS sets are left out: they are no game."""
import sys
import xml.etree.ElementTree as ET

def clean(text):
    return " ".join((text or "").split())

root = ET.parse(sys.argv[1]).getroot()
out = sys.stdout
out.write("# FinalBurn Neo's arcade sets, from its DAT: set, title, model, year, maker\n")
for game in root.iter("game"):
    if game.get("isbios") == "yes":
        continue
    name = game.get("name", "")
    title = clean(game.findtext("description"))
    if not name or not title:
        continue
    year = clean(game.findtext("year"))
    maker = clean(game.findtext("manufacturer"))
    out.write(f"{name}\t{title}\t\t{year}\t{maker}\n")
