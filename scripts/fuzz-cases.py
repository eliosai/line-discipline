#!/usr/bin/env python3
# Capture random pty sessions so `just fuzz` can replay them against the crate
import argparse
import importlib.util
import multiprocessing
import os
import random
import sys
import termios

HERE = os.path.dirname(os.path.abspath(__file__))
SPEC = importlib.util.spec_from_file_location("capture", os.path.join(HERE, "capture-cases.py"))
capture = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(capture)
capture.SETTLE = 0.12

I = capture.I
O = capture.O
L = capture.L
V = capture.V
Case = capture.Case

# (byte, weight): letters and line ends dominate, editing and control bytes stay frequent
ALPHABET = [
    (b"a", 20), (b"b", 10), (b"Z", 5), (b"7", 5), (b" ", 8), (b"_", 3),
    (b"\n", 12), (b"\r", 6), (b"\t", 5), (b"\x7f", 8), (b"\x08", 3),
    (b"\x15", 3), (b"\x17", 3), (b"\x12", 2), (b"\x16", 3), (b"\x04", 3),
    (b"\x03", 2), (b"\x1c", 1), (b"\x1a", 1), (b"\x13", 2), (b"\x11", 2),
    (b"\x00", 1), (b"\x01", 2), (b"\x1b", 1), (b"\xff", 2), (b"\x85", 1),
    (b"\xc3", 2), (b"\xa9", 3), (b"\xe9", 2), (b"\xdf", 1), (b"#", 2), (b";", 2),
]
SIGNALS = {0x03, 0x1C, 0x1A}
OUTPUT = [
    (b"a", 20), (b"B", 5), (b"\n", 10), (b"\r", 8), (b"\t", 8), (b"\x08", 3),
    (b"\x01", 2), (b"\xe9", 2), (b"\xdf", 1), (b"\xc3", 2), (b"\xa9", 2), (b" ", 5),
]


def pick(rng, table, count):
    bytes_, weights = zip(*table)
    return b"".join(rng.choices(bytes_, weights=weights, k=count))


def flag_set(rng, table, names):
    value = 0
    for name, chance in names:
        if rng.random() < chance:
            value |= table[name]
    return value


def random_termios(rng):
    iflag = flag_set(rng, I, [
        ("ICRNL", 0.6), ("IXON", 0.8), ("IXANY", 0.3), ("IGNCR", 0.15), ("INLCR", 0.15),
        ("ISTRIP", 0.2), ("IUCLC", 0.2), ("IUTF8", 0.35), ("PARMRK", 0.2),
    ])
    oflag = flag_set(rng, O, [
        ("OPOST", 0.85), ("ONLCR", 0.7), ("OCRNL", 0.2), ("ONOCR", 0.2), ("ONLRET", 0.2),
        ("OLCUC", 0.2), ("XTABS", 0.3),
    ])
    lflag = flag_set(rng, L, [
        ("ISIG", 0.8), ("ICANON", 0.8), ("ECHO", 0.85), ("ECHOE", 0.75), ("ECHOK", 0.75),
        ("ECHOCTL", 0.75), ("ECHOKE", 0.7), ("IEXTEN", 0.8), ("ECHONL", 0.15),
        ("ECHOPRT", 0.15), ("NOFLSH", 0.15), ("EXTPROC", 0.05),
    ])
    cc = {}
    if rng.random() < 0.3:
        cc[V["VEOL"]] = 0x23
    if rng.random() < 0.2:
        cc[V["VEOL2"]] = 0x3B
    if rng.random() < 0.2:
        cc[V["VERASE"]] = 0x08
    if rng.random() < 0.1:
        cc[V["VLNEXT"]] = 0
    return Case("", [], iflag=iflag, oflag=oflag, lflag=lflag, cc=cc)


def master_write(rng):
    count = rng.randint(1, 60) if rng.random() < 0.9 else rng.randint(60, 200)
    data = bytearray(pick(rng, ALPHABET, count))
    # A signal retracts the echo the master has not read, which races the flip buffer past one
    # 256 byte block, so a write that raises one stays short enough to settle the race
    if any(byte in SIGNALS for byte in data):
        data = data[:40]
    return ("master", bytes(data))


def random_step(rng):
    roll = rng.random()
    if roll < 0.6:
        return master_write(rng)
    if roll < 0.8:
        return ("replica", pick(rng, OUTPUT, rng.randint(1, 40)))
    if roll < 0.9:
        return ("set", random_termios(rng))
    if roll < 0.94:
        return ("tcoff",)
    if roll < 0.97:
        return ("tcon",)
    return ("flush",)


def random_case(seed):
    rng = random.Random(seed)
    base = random_termios(rng)
    steps = [random_step(rng) for _ in range(rng.randint(1, 6))]
    return Case(f"fuzz_{seed}", steps, iflag=base.iflag, oflag=base.oflag, lflag=base.lflag, cc=base.cc)


def racy(lines):
    # A signal retracts echo the master has not read, and past one 256 byte echo block that
    # retraction races the flip buffer, so a step that raises one and echoes that much is dropped
    signal = False
    for line in lines:
        if line.startswith("write master "):
            signal = any(byte in SIGNALS for byte in bytes.fromhex(line.split()[2]))
        elif line.startswith("master ") and signal and len(line) - 7 >= 512:
            return True
    return False


def worker(seed):
    case = random_case(seed)
    runs = []
    base = None
    for _ in range(3):
        out = []
        base = capture.run(case, out)
        runs.append(out)
    keep = runs[0] == runs[1] == runs[2] and not racy(runs[0])
    return (capture.fmt(base), runs[0] if keep else None)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--count", type=int, default=500)
    parser.add_argument("--jobs", type=int, default=os.cpu_count() or 4)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    seeds = [args.seed * 1_000_000 + index for index in range(args.count)]
    with multiprocessing.Pool(args.jobs) as pool:
        results = pool.map(worker, seeds, chunksize=4)
    lines = [f"default {results[0][0]}"]
    kept = 0
    for _, out in results:
        if out is not None:
            lines.extend(out)
            kept += 1
    os.makedirs(os.path.dirname(os.path.abspath(args.out)), exist_ok=True)
    with open(args.out, "w") as handle:
        handle.write("\n".join(lines) + "\n")
    print(f"wrote {kept} of {args.count} cases to {args.out} ({args.count - kept} unstable)", file=sys.stderr)


if __name__ == "__main__":
    main()
