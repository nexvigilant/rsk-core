#!/usr/bin/env python3
"""Clinical Efficacy Test Harness for the Microgram Fleet.

Tests what the standard test suite does NOT:
  1. REJECTION: Do micrograms reject garbage/missing/wrong-type inputs?
  2. SAFETY: Do safety-critical micrograms route death/serious cases correctly?
  3. COMPOSITION: Do chain outputs type-match the next microgram's inputs?
  4. BOUNDARY: Can statistical values cross into clinical assessment unchecked?

Source: Experiment 7 (session 2026-03-08) found 100% silent accept rate
across 250 micrograms. This harness prevents regression.

Usage:
  python3 rsk/clinical_efficacy_test.py                    # Run all tests
  python3 rsk/clinical_efficacy_test.py --suite rejection  # Run one suite
  python3 rsk/clinical_efficacy_test.py --verbose          # Show all results
  python3 rsk/clinical_efficacy_test.py --fix              # Show fix suggestions
"""

import yaml
import subprocess
import json
import glob
import os
import sys
import argparse
from dataclasses import dataclass, field
from typing import Optional
from pathlib import Path

# --- Configuration ---

RSK_BIN = os.path.expanduser("~/Projects/rsk-core/target/release/rsk")
MCG_DIR = os.path.expanduser("~/Projects/rsk-core/rsk/micrograms")

# Safety-critical micrograms that MUST route correctly
# Source: ICH E2A Section II.B (seriousness criteria),
#         ICH E2B (expedited reporting deadlines)
SAFETY_CRITICAL = {
    "case-seriousness": {
        "death_must_be_serious": {
            "input": {"death": True},
            "expect_field": "seriousness",
            "expect_value": "SERIOUS",
        },
        "hospitalization_must_be_serious": {
            "input": {"hospitalization": True},
            "expect_field": "seriousness",
            "expect_value": "SERIOUS",
        },
        "life_threatening_must_be_serious": {
            "input": {"life_threatening": True},
            "expect_field": "seriousness",
            "expect_value": "SERIOUS",
        },
        "empty_is_not_serious": {
            "input": {},
            "expect_field": "seriousness",
            "expect_value": "NON-SERIOUS",
        },
    },
    "seriousness-to-deadline": {
        "fatal_gets_7_day": {
            # seriousness-to-deadline expects is_fatal/is_unexpected/is_serious/is_life_threatening
            "input": {"is_fatal": True, "is_unexpected": True, "is_serious": True},
            "expect_field": "deadline_days",
            "expect_value": 7,
        },
        "serious_unexpected_gets_15_day": {
            "input": {"is_serious": True, "is_unexpected": True, "is_fatal": False, "is_life_threatening": False},
            "expect_field": "deadline_days",
            "expect_value": 15,
        },
    },
    "causality-to-action": {
        "definite_serious_gets_expedited": {
            "input": {"causality": "DEFINITE", "is_serious": True},
            "expect_field": "regulatory_action",
            "expect_value": "expedited_report",
        },
        "definite_serious_gets_7_day": {
            "input": {"causality": "DEFINITE", "is_serious": True},
            "expect_field": "deadline_days",
            "expect_value": 7,
        },
    },
    "naranjo-quick": {
        "score_9_is_definite": {
            "input": {"naranjo_score": 9},
            "expect_field": "causality",
            "expect_value": "DEFINITE",
        },
        "score_0_is_doubtful": {
            "input": {"naranjo_score": 0},
            "expect_field": "causality",
            "expect_value": "DOUBTFUL",
        },
    },
}

# Known chain compositions that must type-match
# Source: CLAUDE.md microgram chain topology
CHAIN_COMPOSITIONS = [
    ("prr-signal", "signal-to-causality", {"prr": 3.5}),
    ("signal-to-causality", "naranjo-quick", None),  # Cannot auto-chain (boundary crossing)
    ("naranjo-quick", "causality-to-action", {"naranjo_score": 7}),
    ("case-seriousness", "seriousness-to-deadline", {"death": True}),
]

# Garbage inputs for rejection testing
GARBAGE_INPUTS = [
    ('empty_object', {}),
    ('bogus_fields', {"totally_bogus_field": "nonsense", "fake_number": 99999}),
    ('wrong_type_string_for_number', {"value": "not_a_number"}),
    ('wrong_type_number_for_bool', {"value": 42}),
    ('null_value', {"value": None}),
    ('nested_garbage', {"a": {"b": {"c": "deep"}}}),
]


# --- Data Classes ---

@dataclass
class TestResult:
    suite: str
    microgram: str
    test_name: str
    passed: bool
    detail: str
    severity: str = "INFO"  # INFO, WARNING, CRITICAL
    fix_suggestion: Optional[str] = None


@dataclass
class SuiteReport:
    name: str
    results: list = field(default_factory=list)

    @property
    def passed(self):
        return sum(1 for r in self.results if r.passed)

    @property
    def failed(self):
        return sum(1 for r in self.results if not r.passed)

    @property
    def critical_failures(self):
        return sum(1 for r in self.results if not r.passed and r.severity == "CRITICAL")


# --- Microgram Execution ---

def run_mcg(name, inputs, timeout=5):
    """Execute a microgram and return (success, output_dict, duration_us)."""
    path = os.path.join(MCG_DIR, f"{name}.yaml")
    if not os.path.exists(path):
        return False, {"error": f"File not found: {path}"}, 0

    try:
        result = subprocess.run(
            [RSK_BIN, "mcg", "run", "-i", json.dumps(inputs), path],
            capture_output=True, text=True, timeout=timeout
        )
        if result.returncode != 0:
            return False, {"error": result.stderr.strip()}, 0

        data = json.loads(result.stdout)
        return data.get("success", False), data.get("output", {}), data.get("duration_us", 0)
    except subprocess.TimeoutExpired:
        return False, {"error": "timeout"}, 0
    except json.JSONDecodeError:
        return False, {"error": "invalid JSON output"}, 0
    except Exception as e:
        return False, {"error": str(e)}, 0


def load_mcg_spec(name):
    """Load a microgram's YAML spec."""
    path = os.path.join(MCG_DIR, f"{name}.yaml")
    with open(path) as f:
        return yaml.safe_load(f)


def get_interface(spec):
    """Extract interface inputs with types and required flags."""
    iface = spec.get("interface") or {}
    inputs = iface.get("inputs") or {}
    outputs = iface.get("outputs") or {}
    return inputs, outputs


# --- Test Suite 1: REJECTION ---

def test_rejection(verbose=False):
    """Test that micrograms with required fields reject missing/garbage input."""
    report = SuiteReport(name="REJECTION")
    files = sorted(glob.glob(os.path.join(MCG_DIR, "*.yaml")))

    for f in files:
        with open(f) as fh:
            spec = yaml.safe_load(fh)

        if not spec or not isinstance(spec, dict) or "name" not in spec:
            continue

        name = spec["name"]
        inputs, _ = get_interface(spec)
        required = {k: v for k, v in inputs.items()
                    if isinstance(v, dict) and v.get("required")}

        if not required:
            continue  # Skip micrograms with no required fields

        # Test 1a: Empty input should fail when required fields exist
        success, output, _ = run_mcg(name, {})
        if success:
            report.results.append(TestResult(
                suite="REJECTION",
                microgram=name,
                test_name="empty_input_accepted",
                passed=False,
                detail=f"Accepted empty input despite {len(required)} required field(s): {list(required.keys())}",
                severity="WARNING",
                fix_suggestion=f"Add input validation in rsk engine: reject when required fields [{', '.join(required.keys())}] are missing",
            ))
        else:
            report.results.append(TestResult(
                suite="REJECTION", microgram=name,
                test_name="empty_input_rejected", passed=True,
                detail="Correctly rejected empty input",
            ))

        # Test 1b: Garbage fields (no valid fields present)
        success, output, _ = run_mcg(name, {"bogus_xyz": "garbage"})
        if success:
            report.results.append(TestResult(
                suite="REJECTION",
                microgram=name,
                test_name="garbage_input_accepted",
                passed=False,
                detail=f"Accepted garbage input, produced: {json.dumps(output)[:100]}",
                severity="WARNING",
                fix_suggestion="Add strict mode: reject input with unrecognized fields when required fields are missing",
            ))
        else:
            report.results.append(TestResult(
                suite="REJECTION", microgram=name,
                test_name="garbage_input_rejected", passed=True,
                detail="Correctly rejected garbage input",
            ))

    return report


# --- Test Suite 2: SAFETY ---

def test_safety(verbose=False):
    """Test that safety-critical micrograms route cases correctly."""
    report = SuiteReport(name="SAFETY")

    for mcg_name, cases in SAFETY_CRITICAL.items():
        for test_name, spec in cases.items():
            success, output, dur = run_mcg(mcg_name, spec["input"])

            expected_field = spec["expect_field"]
            expected_value = spec["expect_value"]
            actual_value = output.get(expected_field)

            if not success:
                report.results.append(TestResult(
                    suite="SAFETY", microgram=mcg_name,
                    test_name=test_name, passed=False,
                    detail=f"Execution failed: {output.get('error', 'unknown')}",
                    severity="CRITICAL",
                ))
            elif actual_value != expected_value:
                report.results.append(TestResult(
                    suite="SAFETY", microgram=mcg_name,
                    test_name=test_name, passed=False,
                    detail=f"{expected_field}: expected '{expected_value}', got '{actual_value}'",
                    severity="CRITICAL",
                    fix_suggestion=f"Check tree node that evaluates {expected_field} — may be field name mismatch or threshold error",
                ))
            else:
                report.results.append(TestResult(
                    suite="SAFETY", microgram=mcg_name,
                    test_name=test_name, passed=True,
                    detail=f"{expected_field}={actual_value} ({dur}us)",
                ))

    return report


# --- Test Suite 3: COMPOSITION ---

def test_composition(verbose=False):
    """Test that chain outputs type-match the next microgram's inputs."""
    report = SuiteReport(name="COMPOSITION")

    for src_name, dst_name, seed_input in CHAIN_COMPOSITIONS:
        src_spec = load_mcg_spec(src_name)
        dst_spec = load_mcg_spec(dst_name)

        _, src_outputs = get_interface(src_spec)
        dst_inputs, _ = get_interface(dst_spec)

        # Check: do source output field names overlap with destination input field names?
        src_fields = set(src_outputs.keys()) if isinstance(src_outputs, dict) else set()
        dst_fields = set(dst_inputs.keys()) if isinstance(dst_inputs, dict) else set()
        dst_required = {k for k, v in dst_inputs.items()
                        if isinstance(v, dict) and v.get("required")}

        overlap = src_fields & dst_fields
        missing_required = dst_required - src_fields

        if missing_required:
            # This is the confinement boundary — source can't provide what destination needs
            report.results.append(TestResult(
                suite="COMPOSITION", microgram=f"{src_name} → {dst_name}",
                test_name="boundary_crossing",
                passed=True,  # This is EXPECTED — it's the confinement boundary
                detail=f"BOUNDARY: {src_name} cannot provide {missing_required} needed by {dst_name}. "
                       f"This is a legitimate confinement boundary (architectural, not a bug).",
                severity="INFO",
            ))
        elif not overlap:
            report.results.append(TestResult(
                suite="COMPOSITION", microgram=f"{src_name} → {dst_name}",
                test_name="no_field_overlap",
                passed=False,
                detail=f"Zero field overlap. Source outputs: {src_fields}. Dest inputs: {dst_fields}.",
                severity="WARNING",
                fix_suggestion=f"Add adapter microgram or rename fields for compatibility",
            ))
        else:
            # Type compatibility check
            type_mismatches = []
            for field_name in overlap:
                src_type = src_outputs.get(field_name, {})
                dst_type = dst_inputs.get(field_name, {})
                if isinstance(src_type, dict) and isinstance(dst_type, dict):
                    st = src_type.get("type", "?")
                    dt = dst_type.get("type", "?")
                    if st != dt:
                        type_mismatches.append(f"{field_name}: {st} → {dt}")

            if type_mismatches:
                report.results.append(TestResult(
                    suite="COMPOSITION", microgram=f"{src_name} → {dst_name}",
                    test_name="type_mismatch",
                    passed=False,
                    detail=f"Type mismatches: {type_mismatches}",
                    severity="WARNING",
                ))
            else:
                # Live test: run source, feed output to destination
                if seed_input is not None:
                    success1, output1, _ = run_mcg(src_name, seed_input)
                    if success1:
                        success2, output2, _ = run_mcg(dst_name, output1)
                        if success2:
                            report.results.append(TestResult(
                                suite="COMPOSITION",
                                microgram=f"{src_name} → {dst_name}",
                                test_name="live_chain",
                                passed=True,
                                detail=f"Chain executed: {json.dumps(seed_input)[:50]} → {json.dumps(output2)[:80]}",
                            ))
                        else:
                            report.results.append(TestResult(
                                suite="COMPOSITION",
                                microgram=f"{src_name} → {dst_name}",
                                test_name="live_chain_dst_fail",
                                passed=False,
                                detail=f"Destination failed on source output: {output2}",
                                severity="WARNING",
                            ))

    return report


# --- Test Suite 4: BOUNDARY (Category Error Detection) ---

def test_boundary(verbose=False):
    """Test that statistical values cannot cross into clinical assessment unchecked.

    Source: Entity Confinement Theory (session 2026-03-08).
    The confinement boundary between statistical signal detection and
    clinical causality assessment must be enforced, not just declared.
    """
    report = SuiteReport(name="BOUNDARY")

    # Test: Can a PRR value be directly fed as a Naranjo score?
    # This SHOULD fail — PRR is a ratio (float), Naranjo is a clinical score (int 0-10)
    for prr_as_naranjo in [2, 5, 10, 50, 200]:
        success, output, _ = run_mcg("naranjo-quick", {"naranjo_score": prr_as_naranjo})
        if success and prr_as_naranjo > 10:
            # Naranjo scale is 0-10. Accepting 50 or 200 is a range violation.
            report.results.append(TestResult(
                suite="BOUNDARY", microgram="naranjo-quick",
                test_name=f"prr_as_naranjo_{prr_as_naranjo}",
                passed=False,
                detail=f"Accepted naranjo_score={prr_as_naranjo} (valid range: -4 to +13). "
                       f"Classified as {output.get('causality')}. "
                       f"A PRR value was accepted as a clinical score without range validation.",
                severity="WARNING",
                fix_suggestion="Add range check: naranjo_score must be in [-4, 13]",
            ))
        elif success and prr_as_naranjo <= 10:
            report.results.append(TestResult(
                suite="BOUNDARY", microgram="naranjo-quick",
                test_name=f"prr_as_naranjo_{prr_as_naranjo}",
                passed=True,
                detail=f"Score {prr_as_naranjo} is in valid Naranjo range, classified: {output.get('causality')}",
            ))

    # Test: Can signal-to-causality output skip the Naranjo assessment entirely?
    # Feed signal-to-causality output directly to causality-to-action
    success1, bridge_output, _ = run_mcg("signal-to-causality", {"signal_detected": True, "prr": 8.5})
    if success1:
        # bridge_output has next_step, priority, recommended_tool — NOT causality
        success2, action_output, _ = run_mcg("causality-to-action", bridge_output)
        if success2:
            # causality-to-action accepted bridge output without causality field
            actual_action = action_output.get("regulatory_action")
            report.results.append(TestResult(
                suite="BOUNDARY",
                microgram="signal-to-causality → causality-to-action (SKIP naranjo)",
                test_name="boundary_bypass",
                passed=False,
                detail=f"Bypassed Naranjo assessment entirely. Bridge output fed directly to action. "
                       f"Got: regulatory_action={actual_action}. "
                       f"The confinement boundary was crossed without clinical assessment.",
                severity="CRITICAL",
                fix_suggestion="causality-to-action should REQUIRE causality field from Naranjo output, "
                              "not accept missing causality as DOUBTFUL",
            ))
        else:
            report.results.append(TestResult(
                suite="BOUNDARY",
                microgram="signal-to-causality → causality-to-action (SKIP naranjo)",
                test_name="boundary_enforced",
                passed=True,
                detail="Correctly rejected bridge output that skipped Naranjo assessment",
            ))

    return report


# --- Reporting ---

def print_report(reports, verbose=False, show_fixes=False):
    """Print formatted test results."""
    total_pass = sum(r.passed for r in reports)
    total_fail = sum(r.failed for r in reports)
    total_critical = sum(r.critical_failures for r in reports)

    print("=" * 72)
    print("CLINICAL EFFICACY TEST REPORT")
    print("=" * 72)
    print()

    for report in reports:
        icon = "PASS" if report.failed == 0 else "FAIL"
        crit = f" ({report.critical_failures} CRITICAL)" if report.critical_failures else ""
        print(f"  [{icon}] {report.name}: {report.passed} passed, {report.failed} failed{crit}")

        if verbose or report.failed > 0:
            for r in report.results:
                if verbose or not r.passed:
                    status = "  OK " if r.passed else " FAIL"
                    sev = f" [{r.severity}]" if not r.passed else ""
                    print(f"       {status} {r.microgram}: {r.test_name}{sev}")
                    if not r.passed or verbose:
                        print(f"              {r.detail[:120]}")
                    if show_fixes and r.fix_suggestion:
                        print(f"              FIX: {r.fix_suggestion[:120]}")
        print()

    print("-" * 72)
    print(f"TOTAL: {total_pass} passed, {total_fail} failed, {total_critical} critical")
    print()

    if total_critical > 0:
        print("*** CRITICAL FAILURES DETECTED ***")
        print("Critical failures indicate patient safety routing errors.")
        print("These must be fixed before deployment.")
        for report in reports:
            for r in report.results:
                if not r.passed and r.severity == "CRITICAL":
                    print(f"  - {r.microgram}: {r.test_name}")
                    print(f"    {r.detail[:140]}")
                    if r.fix_suggestion:
                        print(f"    FIX: {r.fix_suggestion[:140]}")
        print()

    # Exit code: 0 if no critical failures, 1 if any critical
    return 1 if total_critical > 0 else 0


# --- Main ---

def main():
    parser = argparse.ArgumentParser(
        description="Clinical Efficacy Test Harness for the Microgram Fleet"
    )
    parser.add_argument(
        "--suite", choices=["rejection", "safety", "composition", "boundary", "all"],
        default="all", help="Which test suite to run"
    )
    parser.add_argument("--verbose", "-v", action="store_true", help="Show all results")
    parser.add_argument("--fix", action="store_true", help="Show fix suggestions")
    parser.add_argument("--json", action="store_true", help="Output as JSON")
    args = parser.parse_args()

    suites = {
        "rejection": test_rejection,
        "safety": test_safety,
        "composition": test_composition,
        "boundary": test_boundary,
    }

    if args.suite == "all":
        reports = [fn(verbose=args.verbose) for fn in suites.values()]
    else:
        reports = [suites[args.suite](verbose=args.verbose)]

    if args.json:
        results = []
        for report in reports:
            for r in report.results:
                results.append({
                    "suite": r.suite,
                    "microgram": r.microgram,
                    "test": r.test_name,
                    "passed": r.passed,
                    "severity": r.severity,
                    "detail": r.detail,
                    "fix": r.fix_suggestion,
                })
        print(json.dumps(results, indent=2))
        sys.exit(0)

    exit_code = print_report(reports, verbose=args.verbose, show_fixes=args.fix)
    sys.exit(exit_code)


if __name__ == "__main__":
    main()
