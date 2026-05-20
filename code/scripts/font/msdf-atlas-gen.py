#!/usr/bin/env python3

from __future__ import annotations

import os
import sys
import shutil
import subprocess
import zipfile
from pathlib import Path
from urllib.request import urlopen


EXE_NAME = "msdf-atlas-gen.exe"
DOWNLOAD_URL = "https://github.com/Chlumsky/msdf-atlas-gen/releases/download/v1.3/msdf-atlas-gen-1.3-win64.zip"


def download_file(url: str, dest: Path) -> None:
	dest.parent.mkdir(parents=True, exist_ok=True)
	print(f"📥 Downloading {dest.name}...")
	with urlopen(url) as resp, open(dest, "wb") as f:
		shutil.copyfileobj(resp, f)


def extract_zip(zip_path: Path, out_dir: Path) -> set[str]:
	print(f"📦 Extracting {zip_path.name}...")
	with zipfile.ZipFile(zip_path) as zf:
		zf.extractall(out_dir)
		# Determine archive root(s) from members
		roots: set[str] = set()
		for name in zf.namelist():
			top = name.split("/", 1)[0]
			if top:
				roots.add(top)
		return roots


def flatten_if_single_root(out_dir: Path, roots: set[str]) -> None:
	# Only flatten when the zip has a single top-level directory
	if len(roots) != 1:
		return
	root = next(iter(roots))
	candidate = out_dir / root
	if not candidate.is_dir():
		return
	print(f"🧹 Flattening directory...")
	# Move all children (including dotfiles) up one level
	for child in candidate.iterdir():
		target = out_dir / child.name
		if target.exists():
			# Best-effort cleanup if a previous run left files around
			if target.is_dir() and child.is_dir():
				shutil.rmtree(target)
			else:
				target.unlink()
		shutil.move(str(child), str(target))
	# Remove emptied directory
	try:
		candidate.rmdir()
	except OSError:
		pass


def utf8_to_codepoints(s: str) -> str:
	return ",".join(f"0x{ord(c):X}" for c in s)


def main() -> int:
	script_dir = Path(__file__).resolve().parent
	data_dir = script_dir / "../../../data"
	tmp_dir = script_dir / "tmp"
	exe_path = tmp_dir / EXE_NAME

	if not exe_path.exists():
		zip_name = Path(DOWNLOAD_URL).name
		zip_path = tmp_dir / zip_name
		try:
			download_file(DOWNLOAD_URL, zip_path)
			roots = extract_zip(zip_path, tmp_dir)
			flatten_if_single_root(tmp_dir, roots)
		except Exception as e:
			print(f"❌ Error: {e}")
			return 1
		finally:
			# Clean up the downloaded archive
			try:
				if zip_path.exists():
					zip_path.unlink()
			except Exception:
				pass

		print(f"✅ Acquired {EXE_NAME}")

	# https://www.dafont.com/adventure.font
	# https://freefontdownload.org/en/segoe-ui-symbol.font

	subprocess.run(["wine", str(exe_path),
		"-font", str(script_dir / "tmp/adventure/Adventure.otf"),
		"-charset", str(script_dir / "charset.txt"),
		"-and",
		"-font", str(script_dir / "tmp/segoe-ui-symbol_freefontdownload_org/segoe-ui-symbol.ttf"),
		"-charset", str(script_dir / "symbols.txt"),
		"-type", "mtsdf",
		"-format", "png",
		"-potr",
		"-imageout", str(data_dir / "font.png"),
		"-json", str(data_dir / "font.json"),
		"-size", "32",
		"-pxrange", "4",
		"-outerpxpadding", "1"
	])

	return 0


if __name__ == "__main__":
	raise SystemExit(main())
