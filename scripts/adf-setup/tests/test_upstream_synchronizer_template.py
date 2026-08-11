"""Tests for the upstream-synchronizer pick detection wiring.

Validates the fix for terraphim/gitea#51 (parent #43): the drift scan decided
"missing" from SHA reachability alone, so cherry-picks (fresh SHAs) and
[ferrox] adaptations (rebadged subjects) that were already on `main` kept
resurfacing as missing security commits.

These are static contract tests on the real config files -- no Gitea API calls,
no mocks. The behavioural coverage of the detector itself lives in the shell
driver `test_upstream_pick_detect.sh`, which exercises it against real git
repositories.

Both the agent template and the deployed bigbox config are asserted: the
template is the source of truth, the bigbox file is what the box actually runs,
and a fix present in only one of them is not deployed.
"""

from pathlib import Path

try:
    import tomllib
except ImportError:
    import tomli as tomllib  # type: ignore[no-redef]

ADF_SETUP = Path(__file__).parent.parent
TEMPLATE = ADF_SETUP / "agents" / "upstream-synchronizer.toml"
BIGBOX = ADF_SETUP.parent.parent / ".terraphim" / "terraphim.toml.bigbox"
DETECTOR = ADF_SETUP / "upstream-pick-detect.sh"
VERIFIED_PICKS = ADF_SETUP / "gitea-verified-picks.tsv"

DETECTOR_PATH = "/opt/ai-dark-factory/bin/upstream-pick-detect.sh"


def _task_from(path: Path) -> str:
    """Return the upstream-synchronizer task string from a config file."""
    assert path.exists(), f"Missing config: {path}"
    with open(path, "rb") as fh:
        config = tomllib.load(fh)
    for agent in config.get("agents", []):
        if agent.get("name") == "upstream-synchronizer":
            return agent["task"]
    raise AssertionError(f"No upstream-synchronizer agent in {path}")


def _tasks() -> dict:
    return {"template": _task_from(TEMPLATE), "bigbox": _task_from(BIGBOX)}


def test_configs_parse():
    """Both configs must be valid TOML (regression guard for edits)."""
    for path in (TEMPLATE, BIGBOX):
        with open(path, "rb") as fh:
            config = tomllib.load(fh)
        assert config.get("agents"), f"{path} has no agents"


def test_detector_script_is_executable():
    """The detector ships with the mode bit set; the task tests -x before use."""
    assert DETECTOR.exists(), f"Missing detector: {DETECTOR}"
    assert DETECTOR.stat().st_mode & 0o111, f"{DETECTOR} is not executable"


def test_detector_covers_all_three_mechanisms():
    """The three provenance forms issue #51 names must all be implemented."""
    source = DETECTOR.read_text()
    assert "cherry picked from commit" in source, "lost the -x trailer scan"
    assert "Adapted-from:" in source, "lost the [ferrox] trailer scan"
    assert "patch-id --stable" in source, "lost the patch-id fallback"


def test_subject_is_never_a_presence_verdict():
    """A shared subject must not suppress a candidate.

    Subjects collide and generic security wording recurs, so treating one as
    proof would let an unrelated fork commit hide a genuinely missing upstream
    fix. Subject observations are advisory only.
    """
    source = DETECTOR.read_text()
    assert "PRESENT %s subject" not in source, (
        "subject reinstated as a presence mechanism"
    )
    assert "not treated as proof" in source, "lost the subject advisory note"


def test_trailer_refs_are_resolved_not_prefix_matched():
    """Abbreviated trailer refs must be resolved by git, never prefix-compared.

    A prefix comparison lets a short `Adapted-from:` ref vouch for any candidate
    sharing its leading hex digits.
    """
    source = DETECTOR.read_text()
    assert "rev-parse --verify --quiet \"${ref}^{commit}\"" in source, (
        "trailer refs are no longer resolved through git"
    )
    assert "substr(full, 1, n)" not in source, (
        "prefix comparison reinstated in the trailer lookup"
    )


def test_verified_picks_entries_are_revalidated():
    """Attested pairs must be re-checked, not trusted as written."""
    source = DETECTOR.read_text()
    assert "--verified-picks" in source, "lost the attested-pair mechanism"
    assert "is not on ${FORK_REF}, ignoring" in source, (
        "attested fork commits are no longer checked for reachability"
    )


def test_verified_picks_file_entries_resolve():
    """Every shipped attested pair must be two full 40-character object IDs.

    Abbreviations here would reintroduce the ambiguity the trailer fix removed.
    """
    assert VERIFIED_PICKS.exists(), f"Missing attested pairs: {VERIFIED_PICKS}"
    entries = 0
    for raw in VERIFIED_PICKS.read_text().splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        parts = line.split()
        assert len(parts) == 2, f"malformed entry: {raw!r}"
        for sha in parts:
            assert len(sha) == 40, f"not a full object ID: {sha!r}"
            int(sha, 16)  # raises if not hexadecimal
        entries += 1
    assert entries > 0, "attested pairs file has no entries"


def test_candidate_scan_tolerates_no_matches():
    """`grep` exits 1 on no match; under `set -e` that would abort the run."""
    for name, task in _tasks().items():
        idx = task.index("CANDIDATES=$(git log")
        block = task[idx:task.index('if [ -n "$CANDIDATES" ]; then', idx)]
        assert "|| true" in block, (
            f"{name}: candidate scan is unguarded against a no-match grep"
        )


def test_candidates_are_filtered_through_the_detector():
    """The security scan must route candidates through the detector."""
    for name, task in _tasks().items():
        assert DETECTOR_PATH in task, f"{name}: detector not wired in"
        assert "CANDIDATES=" in task, f"{name}: candidate list not captured"
        assert "VERDICTS=" in task, f"{name}: detector output not consumed"
        assert '$1 == "MISSING"' in task, (
            f"{name}: SECURITY_COMMITS must be built from MISSING verdicts only"
        )


def test_missing_detector_aborts_rather_than_falling_back():
    """A missing detector must fail the run.

    Falling back to the SHA-only scan would silently restore the false
    positives that motivated #51 while still appearing to work.
    """
    for name, task in _tasks().items():
        assert f'if [ ! -x "$PICK_DETECT" ]; then' in task, (
            f"{name}: lost the detector presence check"
        )
        assert "exit 1" in task, f"{name}: missing detector does not abort"


def test_no_raw_sha_only_security_scan_remains():
    """The old unfiltered pipeline must be gone from both configs.

    `git log <ref>..upstream/main | grep -iE CVE|security|... | head -10` in one
    expression is the exact construct that produced the #43 false positives.
    """
    for name, task in _tasks().items():
        lines = [ln.strip() for ln in task.splitlines()]
        for i, line in enumerate(lines):
            if line.startswith("SECURITY_COMMITS=$(git log"):
                raise AssertionError(
                    f"{name}: line {i} still assigns SECURITY_COMMITS "
                    f"straight from git log: {line}"
                )


def test_cap_is_applied_after_filtering():
    """`head -10` must follow the MISSING filter, not precede it.

    Capping first would let suppressed picks occupy slots and crowd out
    genuinely missing commits.
    """
    for name, task in _tasks().items():
        filter_at = task.index('$1 == "MISSING"')
        cap_at = task.index("head -10", filter_at)
        assert cap_at > filter_at, f"{name}: cap applied before filtering"


def test_total_suppression_is_distinguishable_in_the_log():
    """"Every candidate already picked" must not read as "nothing matched".

    Filtering introduced a new outcome: candidates are found and all of them
    are suppressed, so SECURITY_COMMITS is empty and the issue-creation gate in
    section 7 takes the informational branch. The standing drift issue then
    stops receiving updates, and this log line is the only record of why.
    """
    for name, task in _tasks().items():
        assert 'elif [ -n "$ALREADY_PICKED" ]; then' in task, (
            f"{name}: total suppression is indistinguishable from no matches"
        )
        assert "already on main -- nothing to report" in task, (
            f"{name}: lost the total-suppression log line"
        )
        assert 'ALREADY_PICKED=""' in task, (
            f"{name}: ALREADY_PICKED must be initialised before the scan"
        )


def test_comparison_is_pinned_to_origin_main():
    """The scan compares against `origin/main`, not the checked-out branch.

    The old scan used `HEAD`, so its output depended on whatever branch the
    fork happened to be left on.
    """
    for name, task in _tasks().items():
        assert "origin/main..upstream/main" in task, (
            f"{name}: drift scan not pinned to origin/main"
        )
        assert "HEAD..upstream/main" not in task, (
            f"{name}: still scanning from HEAD"
        )
