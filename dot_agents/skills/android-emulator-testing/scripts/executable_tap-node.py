#!/usr/bin/env python3
import argparse
import re
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET


def run(command: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, check=False, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)


def parse_bounds(bounds: str) -> tuple[int, int] | None:
    match = re.match(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", bounds)
    if not match:
        return None
    x1, y1, x2, y2 = map(int, match.groups())
    return ((x1 + x2) // 2, (y1 + y2) // 2)


def node_matches(node: ET.Element, args: argparse.Namespace) -> bool:
    text = node.attrib.get("text", "")
    desc = node.attrib.get("content-desc", "")
    if args.text is not None and text != args.text:
        return False
    if args.desc is not None and desc != args.desc:
        return False
    if args.contains is not None and args.contains not in f"{text} {desc}":
        return False
    return True


def main() -> int:
    parser = argparse.ArgumentParser(description="Tap an Android UIAutomator node by text or content description.")
    parser.add_argument("--adb", default="adb")
    parser.add_argument("--serial", default=None)
    parser.add_argument("--text", default=None)
    parser.add_argument("--desc", default=None)
    parser.add_argument("--contains", default=None)
    parser.add_argument("--index", type=int, default=0)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    selectors = [args.text is not None, args.desc is not None, args.contains is not None]
    if not any(selectors):
        parser.error("Provide --text, --desc, or --contains")

    adb_base = [args.adb]
    if args.serial:
        adb_base += ["-s", args.serial]

    dump = run(adb_base + ["shell", "uiautomator", "dump", "/sdcard/window.xml"])
    if dump.returncode != 0:
        sys.stderr.write(dump.stderr or dump.stdout)
        return dump.returncode

    with tempfile.NamedTemporaryFile(suffix=".xml") as handle:
        pull = run(adb_base + ["pull", "/sdcard/window.xml", handle.name])
        if pull.returncode != 0:
            sys.stderr.write(pull.stderr or pull.stdout)
            return pull.returncode
        root = ET.parse(handle.name).getroot()

    matches: list[tuple[ET.Element, tuple[int, int]]] = []
    for node in root.iter("node"):
        if not node_matches(node, args):
            continue
        center = parse_bounds(node.attrib.get("bounds", ""))
        if center is None:
            continue
        matches.append((node, center))

    if args.index >= len(matches):
        sys.stderr.write(f"No matching node at index {args.index}; found {len(matches)} matches\n")
        return 2

    node, (x, y) = matches[args.index]
    print(f"tap x={x} y={y} text={node.attrib.get('text', '')!r} desc={node.attrib.get('content-desc', '')!r}")
    if args.dry_run:
        return 0

    tap = run(adb_base + ["shell", "input", "tap", str(x), str(y)])
    if tap.returncode != 0:
        sys.stderr.write(tap.stderr or tap.stdout)
    return tap.returncode


if __name__ == "__main__":
    raise SystemExit(main())
