"""Trace the native exports into currentColor SVG paths used by the UI.

Usage: python3 scripts/import-sf-symbols.py /path/to/exported/pngs
Requires vtracer; temporary PNG/SVG files stay in the supplied export directory.
"""
import json
from pathlib import Path
import sys
import xml.etree.ElementTree as ET

import vtracer

source = Path(sys.argv[1])
symbols = {}
for image in sorted(source.glob("*.png")):
    svg = image.with_suffix(".svg")
    vtracer.convert_image_to_svg_py(
        str(image), str(svg), colormode="binary", mode="spline",
        filter_speckle=4, path_precision=1, length_threshold=8,
    )
    root = ET.parse(svg).getroot()
    paths = []
    for path in root.findall("{http://www.w3.org/2000/svg}path"):
        paths.append({"d": path.attrib["d"], "transform": path.attrib.get("transform", "")})
    if not paths:
        raise ValueError(f"No paths generated for {image.name}")
    symbols[image.stem] = paths

destination = Path(__file__).resolve().parents[1] / "src/shared/sf-symbols.ts"
destination.write_text(
    "// Generated from Apple SF Symbols, Regular, monochrome. UI use only.\n"
    "// See scripts/export-sf-symbols.swift and scripts/import-sf-symbols.py.\n"
    "export const symbols = " + json.dumps(symbols, ensure_ascii=False, indent=2) + " as const;\n",
    encoding="utf-8",
)
print(f"Generated {len(symbols)} symbols: {destination}")
