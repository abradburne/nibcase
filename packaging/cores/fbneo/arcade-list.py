#!/usr/bin/env python3
"""FinalBurn Neo's arcade sets as DSlot's [games] list, from the core's own data files: its arcade
DAT (ClrMamePro XML: each set's title, year and maker) and its gamelist.txt (each set's hardware).

    arcade-list.py DAT GAMELIST > arcade.tsv

One set a line, tab-separated: set, title, model (the maker, shown first under the title), year,
and series: the board's family, which DSlot's Arcade page has a section for ("Neo Geo", "CPS-2";
empty for the rest, which DSlot puts in Other). BIOS sets are left out: they are no game."""
import re
import sys
import xml.etree.ElementTree as ET

# A board's family, by its hardware as gamelist.txt names it: the first that matches.
FAMILIES = [
    ("Neo Geo", r"^(Neo Geo( MVS| AES)?|dedicated Neo Geo PCB)$"),
    ("CPS-1", r"^(CPS1( / QSound)?|CPS Changer)$"),
    ("CPS-2", r"^CPS2$"),
    ("CPS-3", r"^CPS-3$"),
    ("PGM", r"^PolyGame ?Master"),
    ("Cave", r"^Cave$"),
    ("Toaplan", r"Toaplan"),
    ("Konami", r"^GX\d"),
    ("Sega", r"^(Sega |System (1|2|16[ABC]?|18|24|32|E)$|X-Board|Y-Board|Out Run|Hang-On|G80 |Zaxxon|C2?$)"),
    ("Taito", r"^Taito"),
    ("Irem", r"^Irem"),
    ("Data East", r"^DEC"),
    ("Midway", r"^(MIDWAY|Midway|[TXYZ] Unit$|MCR3$)"),
    ("Kaneko", r"Kaneko|^EXPRO-02$"),
    ("Psikyo", r"^(Psikyo|PS[345])"),
    ("Seta", r"^(Newer )?Seta$|^SSV$"),
]


def clean(text):
    return " ".join((text or "").split())


def family(hardware):
    for name, pattern in FAMILIES:
        if re.search(pattern, hardware):
            return name
    return ""


def hardware_by_set(gamelist):
    found = {}
    for line in open(gamelist, encoding="utf-8", errors="replace"):
        cells = [c.strip() for c in line.split("|")]
        # | name | status | full name | parent | year | company | hardware | remarks |
        if len(cells) >= 9 and cells[1] and cells[1] != "name":
            found[cells[1]] = cells[7]
    return found


def main(dat, gamelist):
    hardware = hardware_by_set(gamelist)
    out = sys.stdout
    out.write("# FinalBurn Neo's arcade sets: set, title, maker, year, board family\n")
    for game in ET.parse(dat).getroot().iter("game"):
        if game.get("isbios") == "yes":
            continue
        name = game.get("name", "")
        title = clean(game.findtext("description"))
        if not name or not title:
            continue
        year = clean(game.findtext("year"))
        maker = clean(game.findtext("manufacturer"))
        out.write(f"{name}\t{title}\t{maker}\t{year}\t{family(hardware.get(name, ''))}\n")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    main(sys.argv[1], sys.argv[2])
