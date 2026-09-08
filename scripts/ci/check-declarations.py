"""Compile static and DLL declarations for both MSVC architectures without an SDK."""

import os
from pathlib import Path
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[2]
SOURCE = r"""
#include <stdint.h>
#include "spandsp/telephony.h"
#include "spandsp/unaligned.h"
_Static_assert(_Alignof(struct __dealign_uint16) == 1, "16-bit packed access");
_Static_assert(_Alignof(struct __dealign_uint32) == 1, "32-bit packed access");
_Static_assert(_Alignof(struct __dealign_uint64) == 1, "64-bit packed access");
#ifdef DEFINE_API
SPAN_DECLARE(int) declaration_probe(void) { return 7; }
SPAN_DECLARE_DATA int data_probe = 11;
#else
SPAN_DECLARE(int) declaration_probe(void);
extern SPAN_DECLARE_DATA int data_probe;
int consume_api(void) { return declaration_probe() + data_probe; }
#endif
"""

with tempfile.TemporaryDirectory(prefix="spandsp-declarations-") as directory:
    source = Path(directory) / "probe.c"
    output = Path(directory) / "probe.ll"
    source.write_text(SOURCE)
    for target in ("x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"):
        for mode, defines in (
            ("static producer", ["SPANDSP_STATIC", "DEFINE_API"]),
            ("static consumer", ["SPANDSP_STATIC"]),
            ("DLL producer", ["LIBSPANDSP_EXPORTS", "DEFINE_API"]),
            ("DLL consumer", []),
        ):
            subprocess.run(
                [os.environ.get("CC", "clang"), f"--target={target}",
                 "-ffreestanding", "-Werror", "-S", "-emit-llvm",
                 "-I", str(ROOT / "spandsp-sys/vendor/src"),
                 *[f"-D{define}" for define in defines],
                 str(source), "-o", str(output)],
                check=True,
            )
            ir = output.read_text()
            if mode.startswith("static"):
                assert "dllimport" not in ir and "dllexport" not in ir, (target, mode)
            else:
                decoration = "dllexport" if mode.endswith("producer") else "dllimport"
                for symbol in ("declaration_probe", "data_probe"):
                    assert any(decoration in line and f"@{symbol}" in line
                               for line in ir.splitlines()), (target, mode, symbol)
            print(f"PASS: {target}: {mode}")
