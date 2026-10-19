"""Guarded USB deployment of Merit Ledger to the identified Waveshare
ESP32-S3-Touch-LCD-1.47. Run from Git Bash (`make flash` / `make update`).

--replace  FIRST INSTALL / REPAVE: erases the whole board (including any
           previous app's data), then writes bootloader, table and app.
--update   App-only update: refuses unless the board already has exactly
           this partition table, so NVS and the mlstate partition survive.
--check    Reads the board's identity and partition table and says what
           --update would do. Writes nothing.

All modes verify the chip, 16 MB flash and base MAC first.
"""
import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EMBUILD = ROOT / "meritledger_rs/.embuild"
ESP_PYTHON = os.environ.get(
    "MERITLEDGER_ESP_PYTHON",
    "C:/Espressif/python_env/idf5.5_py3.11_env/Scripts/python.exe" if os.name == "nt" else sys.executable,
)
# The unit in ../microcontroller/BOARD_SKILL_ESP32_S3_TOUCH_LCD_1_47.md.
EXPECTED_MAC = "28:84:85:b1:70:e4"


def load_manifest() -> list[dict]:
    manifest = json.loads((EMBUILD / "deployment-images.json").read_text())
    if manifest["chip"] != "esp32s3" or manifest["flash_bytes"] != 0x1000000:
        raise SystemExit("Refusing: manifest is not for a 16 MB ESP32-S3")
    images = manifest["images"]
    if [i["address"] for i in images] != [0x0, 0x8000, 0x10000]:
        raise SystemExit("Refusing: unexpected image offsets")
    for image, limit in zip(images, [0x8000, 0x1000, 0x600000]):
        data = Path(image["path"]).read_bytes()
        if len(data) != image["size"] or len(data) > limit or hashlib.sha256(data).hexdigest() != image["sha256"]:
            raise SystemExit("Refusing: an image changed since the build or exceeds its partition; rebuild first")
    print(f"Features: {manifest.get('features', '?')}")
    return images


def identify(base: list[str], expected_mac: str) -> None:
    result = subprocess.run(base + ["flash_id"], check=True, capture_output=True, text=True)
    print(result.stdout, end="")
    # Newer esptool prints an extended "MAC:" line before "BASE MAC:".
    found = re.search(r"^BASE MAC:\s*([0-9a-f:]+)", result.stdout, re.M | re.I) or re.search(
        r"^MAC:\s*([0-9a-f:]+)", result.stdout, re.M | re.I
    )
    if not found or found.group(1).lower() != expected_mac.lower() or "Detected flash size: 16MB" not in result.stdout:
        raise SystemExit("Board MAC / 16 MB flash does not match the selected unit; nothing written")


def table_matches(base: list[str], ours: bytes) -> bool:
    with tempfile.TemporaryDirectory() as tmp:
        current = Path(tmp) / "table.bin"
        subprocess.run(base + ["read_flash", "0x8000", "0x1000", str(current)], check=True, capture_output=True)
        return current.read_bytes()[: len(ours)] == ours


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", required=True)
    parser.add_argument("--expected-mac", default=EXPECTED_MAC)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--replace", action="store_true")
    mode.add_argument("--update", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = parser.parse_args()

    images = load_manifest()
    base = [ESP_PYTHON, "-m", "esptool", "--chip", "esp32s3", "--port", args.port]
    identify(base, args.expected_mac)
    ours = Path(images[1]["path"]).read_bytes()

    if args.check:
        if table_matches(base, ours):
            print("Partition table matches: `make update` would write the app only.")
        else:
            print("Partition table differs: `make update` would refuse; only `make flash` (erases) would install.")
        return

    flash = []
    if args.update:
        if not table_matches(base, ours):
            raise SystemExit("The board's partition table is not Merit Ledger's; use --replace (erases it). Nothing written")
        flash += [hex(images[2]["address"]), images[2]["path"]]
        print("Updating the app only; NVS and mlstate are kept.", flush=True)
    else:
        for image in images:
            flash += [hex(image["address"]), image["path"]]
        print("Replacing this board: erase all flash, then install Merit Ledger.", flush=True)
        subprocess.run(base + ["erase_flash"], check=True)
    subprocess.run(
        base + ["--baud", "921600", "write_flash", "--flash_size", "16MB", "--flash_mode", "dio", "--flash_freq", "80m"] + flash,
        check=True,
    )
    print("Flash verified and board reset. Check /api/v1/status before declaring the deployment done.")


if __name__ == "__main__":
    main()
