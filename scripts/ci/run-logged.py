"""Keep failure diagnostics visible in the check annotations as well as job logs."""

import subprocess
import sys

result = subprocess.run(sys.argv[1:], stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
output = result.stdout.decode(errors="replace")
print(output, end="", flush=True)
if result.returncode:
    tail = output[-24000:]
    # Individual GitHub annotations are truncated, so preserve the tail in
    # small, ordered chunks for both API and UI readers.
    for start in range(0, len(tail), 2500):
        message = tail[start:start + 2500].replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")
        print(f"::error::{message}")
sys.exit(result.returncode)
