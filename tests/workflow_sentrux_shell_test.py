"""Run Sentrux workflow output handling in real Bash, without external services."""

from pathlib import Path
import os
import subprocess
import tempfile
import unittest


WORKFLOW = Path(__file__).resolve().parents[1] / ".github/workflows/sentrux-quality-gate.yml"


def run_block(step):
    """Extract one literal run block, rejecting content outside its indentation."""
    body = WORKFLOW.read_text().split(f"      - name: {step}\n", 1)[1]
    body = body.split("\n      - name:", 1)[0].split("        run: |\n", 1)[1]
    lines = body.splitlines()
    if any(line.strip() and not line.startswith("          ") for line in lines):
        raise ValueError(f"Run block contains unindented content: {step}")
    return "\n".join(line[10:] if line.startswith("          ") else line for line in lines) + "\n"


class SentruxShellTests(unittest.TestCase):
    def output_file(self, script, **variables):
        """Execute a pure workflow fragment against a real file with spaces."""
        with tempfile.TemporaryDirectory(prefix="sentrux workflow ") as folder:
            output = Path(folder) / "step output.txt"
            output.write_text("existing=value\n")
            environment = dict(os.environ, GITHUB_OUTPUT=str(output), **variables)
            subprocess.run(["bash", "-e", "-c", script], env=environment,
                           check=True, capture_output=True, text=True, timeout=10)
            return output.read_text()

    def test_report_extraction_appends_exact_multiline_output(self):
        # Exercise the real parsing/redirect fragment; do not replace the Sentrux binary.
        script = run_block("Run Sentrux Check").split("# Extract quality signal\n", 1)[1]
        for report, quality, count, status in [
            ("Quality: 9600\n✓ 0 violation(s) found", "9600", "0", "pass"),
            ("Quality: 8000\n✗ 2 violation(s) found\nDetail: a * literal [path]", "8000", "2", "fail"),
        ]:
            with self.subTest(status=status):
                self.assertEqual(
                    self.output_file(script, OUTPUT=report),
                    f"existing=value\nquality={quality}\nviolations={count}\nstatus={status}\n"
                    f"output<<EOF\n{report}\nEOF\n",
                )

    def test_baseline_redirect_preserves_zero_empty_and_known_values(self):
        script = run_block("Fetch Baseline (PR only)").split('if [ "$BASELINE"', 1)[1]
        script = 'if [ "$BASELINE"' + script
        for value, expected in [("", "0"), ("0", "0"), ("9600", "9600")]:
            with self.subTest(value=value):
                self.assertEqual(self.output_file(script, BASELINE=value),
                                 f"existing=value\nbaseline={expected}\n")

    def test_delta_boundary_and_output_path(self):
        template = run_block("Calculate Delta (PR only)")
        for current, baseline, delta, degraded in [
            ("9600", "0", "unknown", "false"),
            ("9600", "", "unknown", "false"),
            ("9600", "9600", "0", "false"),
            ("9500", "9600", "-100", "false"),
            ("9499", "9600", "-101", "true"),
        ]:
            with self.subTest(current=current, baseline=baseline):
                script = template.replace("${{ steps.sentrux.outputs.quality }}", current)
                script = script.replace("${{ steps.baseline.outputs.baseline }}", baseline)
                self.assertEqual(self.output_file(script),
                                 f"existing=value\ndelta={delta}\ndegraded={degraded}\n")

    def test_comment_body_stays_inside_literal_run_block(self):
        script = run_block("Post PR Comment")
        self.assertIn("\n**Quality Signal:** $QUALITY / 10000\n", script)
        self.assertIn("\n<details>\n<summary>Full Report</summary>\n", script)


if __name__ == "__main__":
    unittest.main()
