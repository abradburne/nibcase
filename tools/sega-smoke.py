#!/usr/bin/env python3
"""Original, silent MD/GG smoke cartridges with a changing colour each vblank.

These exercise CPU, VDP, frame timing and state continuation, not game compatibility
or worst-case performance. No game code or firmware is included. Sources are listed
in docs/provenance.md. Output ROMs are temporary test artefacts, never packaged.
"""
import pathlib
import struct
import sys


def megadrive():
    rom = bytearray(32768)
    rom[:8] = struct.pack('>II', 0xFFFFFC, 0x200)
    rom[0x100:0x110] = b'SEGA MEGA DRIVE '
    rom[0x120:0x150] = b'NIBCASE COLOUR SMOKE'.ljust(48)
    rom[0x180:0x18E] = b'GM 00000000-00'
    rom[0x190:0x1A0] = b'J'.ljust(16)
    rom[0x1A0:0x1B0] = struct.pack('>IIII', 0, len(rom)-1, 0xFF0000, 0xFFFFFF)
    rom[0x1F0:0x200] = b'JUE'.ljust(16)
    code = bytearray()

    def word(*v):
        code.extend(struct.pack('>' + 'H'*len(v), *v))

    def write(address, value, long=False):
        word(0x23FC if long else 0x33FC)
        if long:
            word(value >> 16, value & 65535)
        else:
            word(value)
        word(address >> 16, address & 65535)

    word(0x46FC, 0x2700)  # MOVE #$2700,SR: no interrupts
    word(0x1039, 0x00A1, 1, 0x0200, 15, 0x670A)  # version nibble; TMSS only on newer hardware
    write(0xA14000, 0x53454741, True)  # TMSS
    write(0xA11100, 0x100)  # hold Z80 bus
    write(0xA11200, 0)
    for register, value in enumerate([4, 4, 0x30, 0x3C, 7, 0x7C, 0, 0, 0, 0, 0,
                                       0, 0x81, 0x3F, 0, 2, 1, 0, 0]):
        write(0xC00004, 0x8000 | register << 8 | value)
    write(0xC00004, 0x40000000, True)
    word(0x7000, 0x323C, 0x3FFF)  # MOVEQ #0,D0; MOVE.W #16383,D1
    word(0x23C0, 0x00C0, 0, 0x51C9, 0xFFF8)  # clear VRAM; DBRA
    write(0xC00004, 0x40000000, True)
    for _ in range(8):
        write(0xC00000, 0x11111111, True)  # solid tile zero, palette index 1
    write(0xC00004, 0xC0000000, True)
    write(0xC00000, 0)
    write(0xC00000, 0x000E)
    write(0xC00004, 0x8144)  # display on, mode 5
    word(0x7200)  # MOVEQ #0,D1
    loop = len(code)
    # Poll vblank rising then falling. Each update changes the tile's red/green/blue.
    word(0x3039, 0x00C0, 4, 0x0800, 3, 0x67F4)
    write(0xC00004, 0xC0020000, True)
    word(0x5241, 0x33C1, 0x00C0, 0)  # ADDQ.W #1,D1; MOVE.W D1,VDP
    word(0x3039, 0x00C0, 4, 0x0800, 3, 0x66F4)
    branch = len(code)
    word(0x6000, (loop - (branch + 2)) & 65535)
    rom[0x200:0x200+len(code)] = code
    checksum = sum(struct.unpack('>'+'H'*((len(rom)-0x200)//2), rom[0x200:])) & 65535
    rom[0x18E:0x190] = struct.pack('>H', checksum)
    return rom


def gamegear():
    code = bytearray([0xF3, 0x31, 0xF0, 0xDF])  # DI; LD SP,$DFF0

    def out(port, value):
        code.extend([0x3E, value, 0xD3, port])  # LD A,value; OUT (port),A

    for register, value in enumerate([4, 0, 0x0E, 0, 0, 0x7E, 0, 0, 0, 0, 0xFF]):
        out(0xBF, value)
        out(0xBF, 0x80 | register)
    out(0xBF, 0)
    out(0xBF, 0x40)
    code.extend([0x01, 0, 0x40])  # LD BC,$4000
    clear = len(code)
    code.extend([0xAF, 0xD3, 0xBE, 0x0B, 0x78, 0xB1, 0x20, 0])
    code[-1] = (clear - len(code)) & 255
    out(0xBF, 0)
    out(0xBF, 0xC0)
    out(0xBE, 0x0F)
    out(0xBE, 0)
    out(0xBF, 0x40)
    out(0xBF, 0x81)
    code.extend([0x16, 0])  # LD D,0 colour counter
    loop = len(code)
    code.extend([0xDB, 0xBF, 0xE6, 0x80, 0x28, 0xFA])  # wait for vblank status
    out(0xBF, 0)
    out(0xBF, 0xC0)
    code.extend([0x14, 0x7A, 0xD3, 0xBE])  # INC D; LD A,D; OUT ($BE),A
    out(0xBE, 4)
    code.extend([0xC3, loop & 255, loop >> 8])
    rom = code + bytearray(32768-len(code))
    rom[0x7FF0:0x7FF8] = b'TMR SEGA'
    rom[0x7FFF] = 0x6C  # international Game Gear, 32 KiB
    checksum = sum(rom[:0x7FF0]) & 65535
    rom[0x7FFA:0x7FFC] = struct.pack('<H', checksum)
    return rom


if __name__ == '__main__':
    if len(sys.argv) != 3 or sys.argv[1] not in ('md', 'gg'):
        raise SystemExit('usage: sega-smoke.py md|gg OUTPUT')
    pathlib.Path(sys.argv[2]).write_bytes(megadrive() if sys.argv[1] == 'md' else gamegear())
