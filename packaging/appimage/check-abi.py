"""Reject incompatible ELF files and libraries that must come from the host."""
import fnmatch
import pathlib
import re
import subprocess
import sys

failed = False
root = pathlib.Path(sys.argv[1])
host_patterns = ["libwayland-*.so*", "libxkbcommon*.so*", "libxcb-xkb.so*", "libgtk-3.so*", "libgdk-3.so*",
                 "libglib-2.0.so*", "libgobject-2.0.so*", "libgio-2.0.so*",
                 "libgmodule-2.0.so*", "libgthread-2.0.so*", "libnotify.so*"]
manifest = root / "usr/share/chipdx/host-libraries.txt"
if manifest.is_file():
    host_patterns.extend(manifest.read_text().splitlines())
fallback = root / "usr/lib/chipdx-x11"
fallback_libraries = {fallback / name for name in (
    "libxkbcommon.so.0", "libxkbcommon-x11.so.0", "libxcb-xkb.so.1",
)}
if root.is_dir():
    for path in sorted(fallback_libraries | {root / "usr/libexec/chipdx/probe-xkbcommon-x11"}):
        if not path.is_file():
            print("Missing X11 fallback file: {}".format(path), file=sys.stderr)
            failed = True
for path in ([root] if root.is_file() else root.rglob("*")):
    if not path.is_file():
        continue
    if path not in fallback_libraries and any(fnmatch.fnmatch(path.name, pattern) for pattern in host_patterns):
        print("{} must be supplied by the host desktop".format(path), file=sys.stderr)
        failed = True
    with path.open("rb") as stream:
        if stream.read(4) != b"\x7fELF":
            continue
    versions = subprocess.check_output(["readelf", "--version-info", str(path)], text=True)
    required = [tuple(map(int, v.split("."))) for v in re.findall(r"GLIBC_(\d+(?:\.\d+)+)", versions)]
    if required and max(required) > (2, 31):
        print("{} requires GLIBC_{}".format(path, ".".join(map(str, max(required)))), file=sys.stderr)
        failed = True
if failed:
    sys.exit(1)
print("Packaged ELF files require at most GLIBC_2.31")
