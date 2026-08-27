#!/usr/bin/env python3
import sys
from pathlib import Path

ROOT_SCRIPT = Path(__file__).resolve().parent.parent.parent.parent / "scripts" / "prepare_packaged_bindings.py"

if __name__ == "__main__":
    import subprocess
    sys.exit(subprocess.run([sys.executable, str(ROOT_SCRIPT)] + sys.argv[1:]).returncode)
