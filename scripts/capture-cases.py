#!/usr/bin/env python3
# Capture n_tty behavior from a real pty into tests/kernel/cases.txt
import os
import pty
import select
import sys
import termios
from dataclasses import dataclass, field

SETTLE = 0.08
PATH = os.path.join(os.path.dirname(__file__), "..", "tests", "kernel", "cases.txt")

I = {n: getattr(termios, n) for n in "ICRNL IXON IXANY IGNCR INLCR ISTRIP IUCLC IUTF8 PARMRK".split()}
O = {n: getattr(termios, n) for n in "OPOST ONLCR OCRNL ONOCR ONLRET OLCUC XTABS".split()}
L = {
    n: getattr(termios, n)
    for n in "ISIG ICANON ECHO ECHOE ECHOK ECHONL ECHOCTL ECHOPRT ECHOKE IEXTEN NOFLSH EXTPROC".split()
}
V = {n: getattr(termios, n) for n in "VEOF VEOL VEOL2 VERASE VKILL VWERASE VREPRINT VLNEXT".split()}

A = b"a" * 4100


@dataclass
class Case:
    name: str
    steps: list
    iflag: int | None = None
    oflag: int | None = None
    lflag: int | None = None
    cc: dict = field(default_factory=dict)


def flags(base, add="", drop=""):
    value = base
    for name in add.split():
        value |= I.get(name, 0) | O.get(name, 0) | L.get(name, 0)
    for name in drop.split():
        value &= ~(I.get(name, 0) | O.get(name, 0) | L.get(name, 0))
    return value


def apply(fd, case):
    attrs = termios.tcgetattr(fd)
    if case.iflag is not None:
        attrs[0] = case.iflag
    if case.oflag is not None:
        attrs[1] = case.oflag
    if case.lflag is not None:
        attrs[3] = case.lflag
    for index, byte in case.cc.items():
        attrs[6][index] = bytes([byte])
    termios.tcsetattr(fd, termios.TCSANOW, attrs)
    return termios.tcgetattr(fd)


def drain(fd):
    segments = [b""]
    while True:
        ready, _, _ = select.select([fd], [], [], SETTLE)
        if not ready:
            return segments
        try:
            chunk = os.read(fd, 65536)
        except BlockingIOError:
            continue
        if chunk == b"":
            segments.append(b"")
            continue
        segments[-1] += chunk


def write(fd, data):
    try:
        return os.write(fd, data)
    except BlockingIOError:
        return 0


def fmt(attrs):
    cc = " ".join(f"{c[0] if isinstance(c, bytes) else c:02x}" for c in attrs[6][:19])
    return f"{attrs[0]:o} {attrs[1]:o} {attrs[2]:o} {attrs[3]:o} {cc}"


def run(case, out):
    master, replica = pty.openpty()
    os.set_blocking(master, False)
    os.set_blocking(replica, False)
    base = termios.tcgetattr(replica)
    attrs = apply(replica, case)
    out.append(f"case {case.name}")
    out.append(f"termios {fmt(attrs)}")
    for step in case.steps:
        kind = step[0]
        if kind == "master":
            os.write(master, step[1])
            out.append(f"write master {step[1].hex()}")
        elif kind == "replica":
            n = write(replica, step[1])
            out.append(f"write replica {step[1].hex()}")
            if n < len(step[1]):
                out.append(f"written {n}")
        elif kind == "set":
            attrs = apply(replica, step[1])
            out.append(f"termios {fmt(attrs)}")
        elif kind == "flush":
            termios.tcflush(replica, termios.TCIFLUSH)
            out.append("flush")
        elif kind in ("tcoff", "tcon"):
            termios.tcflow(replica, termios.TCOOFF if kind == "tcoff" else termios.TCOON)
            out.append(kind)
        echoed = drain(master)
        out.append(f"master {echoed[0].hex()}")
        for index, segment in enumerate(drain(replica)):
            if index:
                out.append("eof")
            out.append(f"replica {segment.hex()}")
    os.close(master)
    os.close(replica)
    return base


BASE_I = termios.ICRNL | termios.IXON
BASE_O = termios.OPOST | termios.ONLCR
BASE_L = flags(0, "ISIG ICANON ECHO ECHOE ECHOK ECHOCTL ECHOKE IEXTEN")


def m(data):
    return ("master", data)


def r(data):
    return ("replica", data)


CASES = [
    Case("canon_line", [m(b"ls\n")]),
    Case("canon_partial_then_newline", [m(b"ls"), m(b"\n")]),
    Case("canon_two_lines_one_write", [m(b"a\nb\n")]),
    Case("canon_cr_icrnl", [m(b"a\r")]),
    Case("canon_cr_igncr", [m(b"a\rb\n")], iflag=flags(BASE_I, "IGNCR")),
    Case("canon_cr_plain", [m(b"a\rb\n")], iflag=flags(BASE_I, drop="ICRNL")),
    Case("canon_inlcr", [m(b"a\n"), m(b"\r")], iflag=flags(BASE_I, "INLCR")),
    Case("canon_nul_is_plain", [m(b"a\x00b\n")]),
    Case("erase_echoe", [m(b"abc\x7f\n")]),
    Case("erase_no_echoe", [m(b"abc\x7f\n")], lflag=flags(BASE_L, drop="ECHOE")),
    Case("erase_ctrl_echoctl", [m(b"a\x01\x7f\n")]),
    Case("erase_ctrl_no_echoctl", [m(b"a\x01\x7f\n")], lflag=flags(BASE_L, drop="ECHOCTL")),
    Case("erase_empty_line", [m(b"\x7f\n")]),
    Case("erase_utf8", [m(b"a\xc3\xa9\x7f\n")], iflag=flags(BASE_I, "IUTF8")),
    Case("erase_utf8_off", [m(b"a\xc3\xa9\x7f\n")]),
    Case("erase_utf8_partial_at_start", [m(b"\xa9\x7f\n")], iflag=flags(BASE_I, "IUTF8")),
    Case("erase_tab", [m(b"ab\tc\x7f\x7f\n")]),
    Case("erase_tab_after_tab", [m(b"\t\t\x7f\n")]),
    Case("erase_tab_canon_column", [r(b"$ "), m(b"\tx\x7f\x7f\n")]),
    Case("erase_tab_xtabs", [r(b"abc"), m(b"\t\x7f\n")], oflag=flags(BASE_O, "XTABS")),
    Case("erase_tab_ctrl_columns", [m(b"\x01\t\x7f\n")]),
    Case("werase", [m(b"foo bar\x17\n")]),
    Case("werase_latin_lead", [m(b"ab \xc3\xa9\x17\n")], iflag=flags(BASE_I, "IUTF8")),
    Case("werase_no_iexten", [m(b"ab\x17\n")], lflag=flags(BASE_L, drop="IEXTEN")),
    Case("werase_underscore", [m(b"a_b c\x17\x17\n")]),
    Case("kill_echoke", [m(b"abc\x15\n")]),
    Case("kill_echok_only", [m(b"abc\x15\n")], lflag=flags(BASE_L, drop="ECHOKE")),
    Case("kill_no_echok", [m(b"abc\x15\n")], lflag=flags(BASE_L, drop="ECHOK ECHOKE")),
    Case("kill_no_echo", [m(b"abc\x15x\n")], lflag=flags(BASE_L, drop="ECHO")),
    Case("reprint", [m(b"ab\x12\n")]),
    Case("reprint_ctrl", [m(b"a\x01\x12\n")]),
    Case("reprint_no_echo", [m(b"ab\x12\n")], lflag=flags(BASE_L, drop="ECHO")),
    Case("lnext", [m(b"a\x16\x03b\n")]),
    Case("lnext_erase", [m(b"\x16\x7f\n")]),
    Case("lnext_no_echoctl", [m(b"\x16\x03\n")], lflag=flags(BASE_L, drop="ECHOCTL")),
    Case("eof_empty", [m(b"\x04")]),
    Case("eof_after_data", [m(b"hi\x04"), m(b"\x04")]),
    Case("eof_then_line", [m(b"\x04ab\n")]),
    Case("eof_twice", [m(b"\x04\x04")]),
    Case("eol", [m(b"ab#c\n")], cc={V["VEOL"]: 0x23}),
    Case("eol2_iexten", [m(b"a;\n")], cc={V["VEOL2"]: 0x3B}),
    Case("eol2_no_iexten", [m(b"a;\n")], lflag=flags(BASE_L, drop="IEXTEN"), cc={V["VEOL2"]: 0x3B}),
    Case("eol_ctrl_echo", [m(b"a\x05\n")], cc={V["VEOL"]: 0x05}),
    Case("echonl", [m(b"ab\n")], lflag=flags(BASE_L, "ECHONL", drop="ECHO")),
    Case("no_echo", [m(b"ab\x7f\n")], lflag=flags(BASE_L, drop="ECHO")),
    Case("echoprt", [m(b"abc\x7f\x7fd\n")], lflag=flags(BASE_L, "ECHOPRT")),
    Case("echoprt_kill", [m(b"abc\x15\n")], lflag=flags(BASE_L, "ECHOPRT")),
    Case("echoprt_utf8", [m(b"\xc3\xa9\x7f\n")], iflag=flags(BASE_I, "IUTF8"), lflag=flags(BASE_L, "ECHOPRT")),
    Case("echoprt_to_empty", [m(b"a\x7f\n")], lflag=flags(BASE_L, "ECHOPRT")),
    Case("signal_intr", [m(b"ab\x03cd\n")]),
    Case("signal_noflsh", [m(b"ab\x03cd\n")], lflag=flags(BASE_L, "NOFLSH")),
    Case("signal_quit", [m(b"ab\x1ccd\n")]),
    Case("signal_susp", [m(b"ab\x1acd\n")]),
    Case("signal_no_isig", [m(b"a\x03\n")], lflag=flags(BASE_L, drop="ISIG")),
    Case("signal_flushes_completed_line", [m(b"ls\n\x03")]),
    Case("signal_no_echo", [m(b"a\x03b\n")], lflag=flags(BASE_L, drop="ECHO")),
    Case("signal_after_erasing", [m(b"ab\x7f\x03c\n")], lflag=flags(BASE_L, "ECHOPRT")),
    Case("signal_separate_writes", [m(b"ab"), m(b"\x03"), m(b"\n")]),
    Case("flow_stop_start", [m(b"\x13"), r(b"hello\n"), m(b"\x11"), r(b"hello\n")]),
    Case("flow_ixany", [m(b"\x13"), m(b"x"), r(b"y")], iflag=flags(BASE_I, "IXANY")),
    Case("flow_no_ixon", [m(b"\x13\x11\n")], iflag=flags(BASE_I, drop="IXON")),
    Case("flow_echo_held", [m(b"\x13ab"), m(b"\x11")]),
    Case("flow_intr_restarts", [m(b"\x13"), m(b"\x03"), r(b"x")]),
    Case("flow_lnext_literal", [m(b"\x16\x13\n"), r(b"x")]),
    Case("istrip", [m(b"\xe1\n")], iflag=flags(BASE_I, "ISTRIP")),
    Case("iuclc", [m(b"AB\xc9\n")], iflag=flags(BASE_I, "IUCLC")),
    Case("iuclc_no_iexten", [m(b"AB\n")], iflag=flags(BASE_I, "IUCLC"), lflag=flags(BASE_L, drop="IEXTEN")),
    Case("parmrk", [m(b"\xff\n")], iflag=flags(BASE_I, "PARMRK")),
    Case("extproc", [m(b"ab\n")], lflag=flags(BASE_L, "EXTPROC")),
    Case("raw", [m(b"ab\x03\x7f\n")], iflag=0, oflag=0, lflag=0),
    Case("noncanon_echo", [m(b"ab\x7f\n")], lflag=flags(BASE_L, drop="ICANON")),
    Case("noncanon_isig", [m(b"a\x03b")], lflag=flags(BASE_L, drop="ICANON")),
    Case("noncanon_cr", [m(b"a\r")], lflag=flags(BASE_L, drop="ICANON")),
    Case("noncanon_ctrl_echo", [m(b"\x01\n")], lflag=flags(BASE_L, drop="ICANON")),
    Case("canon_max_line", [m(A + b"\n")]),
    Case("canon_max_erase", [m(A[:4096] + b"\x7f\n")]),
    Case("canon_max_exact", [m(A[:4095] + b"\n")]),
    Case("high_byte_echo", [m(b"\x85\xa0\xc0\n")]),
    Case("out_onlcr", [r(b"a\nb")]),
    Case("out_no_opost", [r(b"a\nb\t")], oflag=0),
    Case("out_ocrnl", [r(b"a\r")], oflag=flags(BASE_O, "OCRNL")),
    Case("out_onocr", [r(b"\r\ra\r")], oflag=flags(BASE_O, "ONOCR")),
    Case("out_onlret", [r(b"a\n\r")], oflag=flags(BASE_O, "ONLRET ONOCR", drop="ONLCR")),
    Case("out_xtabs", [r(b"ab\tc")], oflag=flags(BASE_O, "XTABS")),
    Case("out_tab_after_cr", [r(b"abc\r\t")], oflag=flags(BASE_O, "XTABS")),
    Case("out_tab_after_nl", [r(b"abc\n\t")], oflag=flags(BASE_O, "XTABS")),
    Case("out_tab_after_nl_no_onlcr", [r(b"abc\n\t")], oflag=flags(BASE_O, "XTABS", drop="ONLCR")),
    Case("out_olcuc", [r(b"abc\xe9\xf7\xdf\n")], oflag=flags(BASE_O, "OLCUC")),
    Case("out_backspace", [r(b"abc\b\t")], oflag=flags(BASE_O, "XTABS")),
    Case("out_utf8_column", [r(b"\xc3\xa9\t")], iflag=flags(BASE_I, "IUTF8"), oflag=flags(BASE_O, "XTABS")),
    Case("out_utf8_off_column", [r(b"\xc3\xa9\t")], oflag=flags(BASE_O, "XTABS")),
    Case("out_ctrl_column", [r(b"a\x01\t")], oflag=flags(BASE_O, "XTABS")),
    Case("out_ocrnl_onlret", [r(b"ab\r\t")], oflag=flags(BASE_O, "OCRNL ONLRET XTABS")),
    Case("out_ocrnl_no_onlret", [r(b"ab\r\t")], oflag=flags(BASE_O, "OCRNL XTABS")),
    Case("echo_column_interplay", [r(b"abc"), m(b"\t\x7f\n")], oflag=flags(BASE_O, "XTABS")),
    Case("echo_no_opost", [m(b"a\x01\n")], oflag=0),
    Case("switch_to_raw_releases_line", [m(b"ab"), ("set", Case("", [], lflag=flags(BASE_L, drop="ICANON")))]),
    Case("switch_to_canon_keeps", [m(b"ab"), ("set", Case("", [], lflag=BASE_L)), m(b"c\n")], lflag=flags(BASE_L, drop="ICANON")),
    Case("switch_ixon_off_restarts", [m(b"\x13"), ("set", Case("", [], iflag=flags(BASE_I, drop="IXON"))), r(b"x")]),
    Case("switch_ixon_off_releases_echo", [m(b"\x13ab"), ("set", Case("", [], iflag=flags(BASE_I, drop="IXON")))]),
    Case("tcflow_off_holds_output", [("tcoff",), r(b"x"), ("tcon",), r(b"y")]),
    Case("tcflow_off_ignores_vstart", [("tcoff",), m(b"\x11"), r(b"x"), ("tcon",), r(b"y")]),
    Case("tcflow_off_ignores_ixany", [("tcoff",), m(b"a"), r(b"x")], iflag=flags(BASE_I, "IXANY")),
    Case("tcflow_off_ignores_ixon_drop", [("tcoff",), ("set", Case("", [], iflag=flags(BASE_I, drop="IXON"))), r(b"x")]),
    Case("tcflow_on_releases_vstop", [m(b"\x13"), ("tcoff",), ("tcon",), r(b"x")]),
    Case("tcflow_off_echo_waits", [("tcoff",), m(b"ab"), ("tcon",), m(b"c")]),
    Case("tcflow_off_intr_keeps_stopped", [("tcoff",), m(b"\x03"), r(b"x")]),
    Case("switch_resets_lnext", [m(b"\x16"), ("set", Case("", [], lflag=flags(BASE_L, drop="ICANON"))), m(b"\x03")]),
    Case("switch_same_keeps_line", [m(b"ab"), ("set", Case("", [], lflag=BASE_L)), m(b"\n")]),
    Case("flush_input", [m(b"ab"), ("flush",), m(b"\n")]),
    Case("flush_resets_erasing", [m(b"ab\x7f"), ("flush",), m(b"c\n")], lflag=flags(BASE_L, "ECHOPRT")),
    Case("echoprt_newline_keeps_erasing", [m(b"ab\x7f\nc\n")], lflag=flags(BASE_L, "ECHOPRT")),
    Case("flow_echo_before_stop", [m(b"ab\x13c"), m(b"\x11")]),
    Case("istrip_makes_a_signal", [m(b"a\x83b\n")], iflag=flags(BASE_I, "ISTRIP")),
    Case("switch_extproc_releases_line", [m(b"ab"), ("set", Case("", [], lflag=flags(BASE_L, "EXTPROC")))]),
    Case("noncanon_echonl", [m(b"a\n")], lflag=flags(BASE_L, "ECHONL", drop="ECHO ICANON")),
    Case("canon_echonl_icrnl", [m(b"a\r")], lflag=flags(BASE_L, "ECHONL", drop="ECHO")),
    Case("kill_utf8", [m(b"\xc3\xa9\x15\n")], iflag=flags(BASE_I, "IUTF8")),
    Case("werase_ctrl", [m(b"a\x01\x17\n")]),
    Case("reprint_after_erasing", [m(b"ab\x7f\x12\n")], lflag=flags(BASE_L, "ECHOPRT")),
    Case("lnext_after_erasing", [m(b"ab\x7f\x16c\n")], lflag=flags(BASE_L, "ECHOPRT")),
    Case("signal_stopped_noflsh", [m(b"\x13ab\x03")], lflag=flags(BASE_L, "NOFLSH")),
    Case("iuclc_lnext", [m(b"\x16A\n")], iflag=flags(BASE_I, "IUCLC")),
    Case("erase_after_eof", [m(b"ab\x04\x7f\n")]),
    Case("two_eof_lines", [m(b"a\x04\x04b\n")]),
    Case("signals_in_a_row", [m(b"\x03"), m(b"\x1c"), m(b"\x1a")]),
    Case("parmrk_eol", [m(b"a\xff")], iflag=flags(BASE_I, "PARMRK"), cc={V["VEOL"]: 0xFF}),
    Case("out_no_opost_no_column", [r(b"ab"), m(b"\t\x7f\n")], oflag=0),
    Case("echo_escape_column_no_opost", [m(b"\xff\n\t\x7f\n")], oflag=0),
    Case("eol_at_line_limit", [m(A[:4096] + b"#")], cc={V["VEOL"]: 0x23}),
]


def main():
    out = []
    base = None
    for case in CASES:
        base = run(case, out)
    header = [f"default {fmt(base)}"]
    with open(PATH, "w") as handle:
        handle.write("\n".join(header + out) + "\n")
    print(f"wrote {len(CASES)} cases to {os.path.relpath(PATH)}", file=sys.stderr)


if __name__ == "__main__":
    main()
