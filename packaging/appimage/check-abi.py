"""Reject any packaged ELF requiring glibc newer than our build baseline."""
import pathlib
import re
import subprocess
import sys

failed = False
root = pathlib.Path(sys.argv[1])
for path in ([root] if root.is_file() else root.rglob("*")):
    if not path.is_file():
        continue
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
