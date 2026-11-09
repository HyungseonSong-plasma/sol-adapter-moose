#!/usr/bin/env python3
"""Verify one native steady-thermal scalar+field evidence pair; no SOL 0.3 producer.

Read only archived or live adapter V&V bytes. SciPy is a test-only dependency.
This script does not authorize source mappings, canonical observation IDs,
physical time semantics, dataset retention or scientific release approval.
"""
from __future__ import annotations

import argparse
import csv
from hashlib import sha256
from io import BytesIO, StringIO
import json
import math
from pathlib import Path
import re
import zipfile

from scipy.io import netcdf_file

RUN_KEY = "thermal-constant-field-vv"
REL = "phase5-vv-evidence/runs/" + RUN_KEY + "/"
SUMMARY = "phase5-vv-evidence/thermal-vv.json"
FILES = ("input.i", "input_out.csv", "input_out.e", "stdout.log", "stderr.log")


def _finite(value: object) -> float:
    result = float(value)
    if not math.isfinite(result):
        raise ValueError("nonfinite numerical evidence")
    return result


def _read_sources(archive: Path | None, root: Path | None) -> dict[str, bytes]:
    if (archive is None) == (root is None):
        raise ValueError("provide exactly one of --archive or --evidence-root")
    if archive is not None:
        with zipfile.ZipFile(archive) as z:
            if z.testzip() is not None:
                raise ValueError("corrupt ZIP source")
            names = z.namelist()
            required = [SUMMARY, *(REL + item for item in FILES)]
            if any(names.count(name) != 1 for name in required):
                raise ValueError("missing native same-run source member")
            return {name: z.read(name) for name in required}
    assert root is not None
    root = root.resolve()
    required = [SUMMARY, *(REL + item for item in FILES)]
    return {name: (root.parent / name).read_bytes() for name in required}


def _csv_last(data: bytes) -> tuple[float, float, float]:
    records = list(csv.DictReader(StringIO(data.decode("utf-8"))))
    if len(records) < 2:
        raise ValueError("CSV lacks initial and terminal sample")
    for row in records:
        if set(row) != {"time", "temperature_max_vv", "temperature_min_vv"}:
            raise ValueError("unexpected CSV output columns")
    final = records[-1]
    return (_finite(final["time"]), _finite(final["temperature_min_vv"]),
            _finite(final["temperature_max_vv"]))


def _field(data: bytes) -> dict:
    if not data.startswith((b"CDF\x01", b"CDF\x02")):
        raise ValueError("expected original classic NetCDF Exodus bytes")
    with netcdf_file(BytesIO(data), mode="r", mmap=False) as dataset:
        names = dataset.variables["name_nod_var"][:].copy()
        variable_names = [row.tobytes().split(b"\x00", 1)[0].decode("ascii")
                          for row in names]
        if variable_names != ["T"]:
            raise ValueError("native Exodus is not a single nodal T field")
        values = dataset.variables["vals_nod_var1"][:].copy()
        times = dataset.variables["time_whole"][:].copy()
        x = dataset.variables["coordx"][:].copy()
        y = dataset.variables["coordy"][:].copy()
        z = dataset.variables["coordz"][:].copy()
        connectivity = dataset.variables["connect1"][:].copy()
    if (values.ndim != 2 or values.shape != (len(times), 5)
            or len(times) < 2 or len(x) != 5 or len(y) != 5 or len(z) != 5
            or connectivity.shape != (4, 2)):
        raise ValueError("unexpected 1D LINE2 source topology")
    if (list(map(lambda value: int(value), connectivity.flatten()))
            != [1, 2, 2, 3, 3, 4, 4, 5]):
        raise ValueError("unexpected source connectivity")
    coordinates = [_finite(value) for value in x]
    if any(abs(value - index * 0.25) > 1e-12
           for index, value in enumerate(coordinates)):
        raise ValueError("unexpected source coordinate grid")
    if any(abs(_finite(value)) > 1e-12 for value in (*y, *z)):
        raise ValueError("native source does not have zero transverse coordinates")
    last = [_finite(value) for value in values[-1]]
    return {
        "last_native_step_index": len(times) - 1,
        "last_native_step_label_not_physical_time": _finite(times[-1]),
        "node_count": 5,
        "element_count": 4,
        "variable": "T",
        "association": "NODAL",
        "x_coordinates_native": coordinates,
        "final_nodal_values_native": last,
        "final_nodal_min": min(last),
        "final_nodal_max": max(last),
    }


def verify(archive: Path | None, evidence_root: Path | None) -> dict:
    sources = _read_sources(archive, evidence_root)
    summary = json.loads(sources[SUMMARY])
    if summary["canonical_result_semantics_claimed"] is not False:
        raise ValueError("candidate unexpectedly claims canonical result semantics")
    if summary["benchmark"] != "steady_1d_constant_temperature_natural_zero_flux":
        raise ValueError("unexpected physics benchmark")
    tolerance = _finite(summary["tolerance_k"])
    if not 0 < tolerance <= 1e-8:
        raise ValueError("incompatible source V&V tolerance")
    original_input = sources[REL + "input.i"].decode("utf-8")
    if not (re.search(r"(?m)^\s*dim\s*=\s*1\s*$", original_input)
            and re.search(r"(?m)^\s*nx\s*=\s*4\s*$", original_input)
            and re.search(r"(?m)^\s*type\s*=\s*Steady\s*$", original_input)
            and "type = NodalExtremeValue" in original_input
            and "exodus = true" in original_input
            and "csv = true" in original_input):
        raise ValueError("source input was not the declared steady-thermal V&V run")
    if "Solve Converged!" not in sources[REL + "stdout.log"].decode("utf-8"):
        raise ValueError("native log lacks converge witness")
    if not sources[REL + "input_out.e"]:
        raise ValueError("missing field source bytes")
    last_csv_time, csv_min, csv_max = _csv_last(sources[REL + "input_out.csv"])
    field = _field(sources[REL + "input_out.e"])
    if abs(last_csv_time - field["last_native_step_label_not_physical_time"]) > 1e-12:
        raise ValueError("scalar/field final native step does not match")
    if (abs(csv_min - field["final_nodal_min"]) > tolerance
            or abs(csv_max - field["final_nodal_max"]) > tolerance):
        raise ValueError("scalar/field numerical extrema differ")
    if max(abs(csv_min - 400.0), abs(csv_max - 400.0)) > tolerance:
        raise ValueError("scalar result outside predeclared V&V benchmark")
    if abs(_finite(summary["observed_min_temperature_k"]) - csv_min) > tolerance:
        raise ValueError("V&V JSON/CSV minimum disagree")
    if abs(_finite(summary["observed_max_temperature_k"]) - csv_max) > tolerance:
        raise ValueError("V&V JSON/CSV maximum disagree")
    return {
        "schema": "adapter.native-thermal-same-run-source-evidence.v0.1",
        "status": "NATIVE_SOURCE_SCALAR_FIELD_PAIR_VERIFIED",
        "canonical_result_semantics": "NOT_EVALUATED",
        "science_owner_signoff": "PENDING",
        "sol_validator_approval": "PENDING",
        "adapter_protocol_result_0_3": "NOT_IMPLEMENTED",
        "run_key": RUN_KEY,
        "native_step_semantics": "INDEX_ONLY_NOT_PHYSICAL_TIME",
        "output_source": {
            name: {"sha256": sha256(sources[REL + name]).hexdigest(),
                   "bytes": len(sources[REL + name])}
            for name in FILES
        },
        "vv_summary_sha256": sha256(sources[SUMMARY]).hexdigest(),
        "csv_terminal": {"raw_step_label": last_csv_time, "min": csv_min, "max": csv_max},
        "exodus_terminal": field,
        "maximum_absolute_scalar_field_difference": max(
            abs(csv_min - field["final_nodal_min"]),
            abs(csv_max - field["final_nodal_max"]),
        ),
        "tolerance_k_source_vv_only": tolerance,
        "limitations": [
            "native backend data does not prove canonical quantity/unit/scope",
            "single archive/workspace source path is a provenance claim bounded by CI",
            "MOOSE steady step label is not accepted physical time",
            "no canonical SOL 0.3 field dataset or storage resolver is created",
            "manual independent Physics and SOL signoff remain required",
        ],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--archive", type=Path)
    group.add_argument("--evidence-root", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = verify(args.archive, args.evidence_root)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n",
                           encoding="utf-8")
    print("THERMAL_SAME_RUN_SOURCE_PAIR=" + report["status"])
    print("SOL_CANONICAL_RESULT=" + report["canonical_result_semantics"])


if __name__ == "__main__":
    main()
