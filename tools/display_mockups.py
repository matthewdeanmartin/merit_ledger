# /// script
# requires-python = ">=3.11"
# dependencies = ["pillow>=11"]
# ///
"""Mockups of every card and menu screen on the board, for review.

Unlike gamematcher's (which drew lookalikes in Python), these come from
the firmware's own renderer: `cargo test preview` draws each sample card
(`screen::tests::samples`) to a BMP exactly as the board would, and this
script saves them as PNGs at 3x and tiles them into one contact sheet.

    uv run tools/display_mockups.py            # -> spec/display/
    uv run tools/display_mockups.py some/dir
"""

import os
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
SCALE = 3
GAP = 24
COLS = 3


def render(into: Path) -> list[Path]:
    env = dict(os.environ, ML_PREVIEW=str(into))
    subprocess.run(
        ["cargo", "test", "--lib", "--quiet", "screen::tests::preview"],
        cwd=ROOT / "meritledger_rs",
        env=env,
        check=True,
    )
    return sorted(into.glob("*.bmp"))


def main() -> int:
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "spec" / "display"
    out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        bmps = render(Path(tmp))
        if not bmps:
            print("no cards rendered", file=sys.stderr)
            return 1
        shots = []
        for bmp in bmps:
            img = Image.open(bmp).convert("RGB")
            big = img.resize((img.width * SCALE, img.height * SCALE), Image.Resampling.NEAREST)
            png = out / (bmp.stem + ".png")
            big.save(png)
            shots.append((bmp.stem, img))
    w, h = shots[0][1].size
    rows = (len(shots) + COLS - 1) // COLS
    label = 18
    sheet = Image.new("RGB", (COLS * (w * 2 + GAP) + GAP, rows * (h * 2 + GAP + label) + GAP), (40, 40, 40))
    draw = ImageDraw.Draw(sheet)
    for i, (name, img) in enumerate(shots):
        x = GAP + (i % COLS) * (w * 2 + GAP)
        y = GAP + (i // COLS) * (h * 2 + GAP + label)
        draw.text((x, y), name, fill=(220, 220, 220))
        sheet.paste(img.resize((w * 2, h * 2), Image.Resampling.NEAREST), (x, y + label))
    sheet.save(out / "sheet.png")
    print(f"{len(shots)} screens -> {out} (sheet.png has them all)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
