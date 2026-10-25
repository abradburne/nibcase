#!/usr/bin/env python3
"""Build an original NROM test cartridge, not an emulator or a commercial game.

Checkerboard video, a quiet pulse tone, controller bits at SRAM byte 1, and a
persistent frame counter at SRAM byte 0. Hardware sources: docs/provenance.md.
The generated .nes is test output; it is never shipped in a package.
"""
import pathlib
import struct
import sys


def build():
    code = bytearray()
    labels, fixes = {}, []

    def emit(*values):
        code.extend(values)

    def absolute(op, address):
        emit(op, address & 255, address >> 8)

    def store(address, value):
        emit(0xA9, value)  # LDA #value
        absolute(0x8D, address)  # STA address

    def mark(name):
        labels[name] = len(code)

    def branch(op, name):
        emit(op, 0)
        fixes.append((len(code) - 1, name, True))

    def jump(name):
        emit(0x4C, 0, 0)
        fixes.append((len(code) - 2, name, False))

    emit(0x78, 0xD8, 0xA2, 0xFF, 0x9A)  # SEI, CLD, LDX #$ff, TXS
    for address in [0x2000, 0x2001, 0x4010]:
        store(address, 0)
    store(0x4017, 0x40)  # inhibit frame IRQ
    for name in ["warm1", "warm2"]:
        mark(name)
        absolute(0x2C, 0x2002)  # BIT status; wait for two vblanks
        branch(0x10, name)  # BPL
    store(0x2006, 0x3F)
    store(0x2006, 0)
    emit(0xA2, 0)  # X palette byte
    mark("palette")
    emit(0x8A, 0x29, 0x03, 0x09, 0x10)  # TXA, AND #3, ORA #$10
    absolute(0x8D, 0x2007)
    emit(0xE8, 0xE0, 32)  # INX, CPX #32
    branch(0xD0, "palette")
    store(0x2006, 0x20)
    store(0x2006, 0)
    emit(0xA0, 4, 0xA2, 0, 0xA9, 1)  # four pages of tile 1
    mark("nametable")
    absolute(0x8D, 0x2007)
    emit(0xE8)
    branch(0xD0, "nametable")
    emit(0x88)  # DEY
    branch(0xD0, "nametable")
    store(0x2005, 0)
    store(0x2005, 0)
    store(0x2001, 0x0A)
    for address, value in [(0x4015, 1), (0x4000, 0xB3), (0x4001, 0),
                           (0x4002, 0xFD), (0x4003, 0x08)]:
        store(address, value)
    mark("frame")
    absolute(0x2C, 0x2002)
    branch(0x10, "frame")
    absolute(0xEE, 0x6000)  # persistent frame counter
    store(0x4016, 1)
    store(0x4016, 0)
    store(0x6001, 0)
    emit(0xA2, 8)
    mark("buttons")
    absolute(0xAD, 0x4016)
    emit(0x4A)  # LSR A: controller D0 -> carry
    absolute(0x2E, 0x6001)  # ROL SRAM byte (A B Select Start Up Down Left Right)
    emit(0xCA)
    branch(0xD0, "buttons")
    jump("frame")
    mark("interrupt")
    emit(0x40)  # RTI; interrupts stay disabled
    for offset, name, relative in fixes:
        if relative:
            delta = labels[name] - (offset + 1)
            assert -128 <= delta < 128
            code[offset] = delta & 255
        else:
            code[offset:offset + 2] = struct.pack("<H", 0x8000 + labels[name])
    prg = code + bytearray(16384 - len(code))
    prg[-6:] = struct.pack("<HHH", 0x8000 + labels["interrupt"], 0x8000,
                          0x8000 + labels["interrupt"])
    chr_rom = bytearray(8192)
    chr_rom[16:24] = bytes([0xAA, 0x55] * 4)
    header = b"NES\x1a" + bytes([1, 1, 2, 0, 1]) + bytes(7)
    return header + prg + chr_rom


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: nes-smoke.py OUTPUT.nes")
    pathlib.Path(sys.argv[1]).write_bytes(build())
