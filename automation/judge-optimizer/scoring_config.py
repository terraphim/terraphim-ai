"""
Judge Scoring Configuration Optimizer

Generates variations of scoring thresholds and replays verdicts to find optimal configurations.
"""

import json
from dataclasses import dataclass, asdict
from itertools import product
from pathlib import Path
from typing import List, Dict, Tuple
import random


@dataclass
class ScoringConfig:
    """Scoring thresholds for judge verdicts"""

    name: str
    accept_min_dimension: int  # All dimensions must be >= this for accept
    accept_min_average: float  # Average must be >= this for accept
    improve_min_dimension: int  # All dimensions must be >= this for improve
    reject_max_dimension: int  # Any dimension < this triggers reject

    def to_dict(self) -> dict:
        return asdict(self)

    def get_verdict(self, scores: Dict[str, int]) -> Tuple[str, float]:
        """Apply scoring rules to determine verdict"""
        dims = ["semantic", "pragmatic", "syntactic"]
        values = [scores.get(d, 3) for d in dims]
        avg = sum(values) / len(values)

        # Reject: any dimension too low
        if any(v < self.reject_max_dimension for v in values):
            return "reject", avg

        # Improve: not good enough for accept but not rejection-worthy
        if (
            any(v < self.accept_min_dimension for v in values)
            or avg < self.accept_min_average
        ):
            if all(v >= self.improve_min_dimension for v in values):
                return "improve", avg

        # Accept: all conditions met
        if (
            all(v >= self.accept_min_dimension for v in values)
            and avg >= self.accept_min_average
        ):
            return "accept", avg

        # Default fallback
        return "improve", avg


class ConfigOptimizer:
    """Generates and evaluates scoring configuration variations"""

    # Baseline configuration (current production values)
    BASELINE = ScoringConfig(
        name="baseline",
        accept_min_dimension=3,
        accept_min_average=3.5,
        improve_min_dimension=2,
        reject_max_dimension=2,
    )

    def __init__(self, verdicts_path: Path):
        self.verdicts_path = verdicts_path
        self.verdicts = self._load_verdicts()

    def _load_verdicts(self) -> List[Dict]:
        """Load verdict records from JSONL"""
        verdicts = []
        with open(self.verdicts_path) as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    v = json.loads(line)
                    if "scores" in v and "verdict" in v:
                        verdicts.append(v)
                except json.JSONDecodeError:
                    continue
        return verdicts

    def generate_variations(
        self, count: int = 100, seed: int = 42
    ) -> List[ScoringConfig]:
        """Generate scoring configuration variations

        Strategy: Systematic grid search + random perturbations
        """
        random.seed(seed)
        configs = []

        # Systematic variations (grid search)
        accept_dims = [2, 3, 4]
        accept_avgs = [3.0, 3.25, 3.5, 3.75, 4.0]
        improve_mins = [1, 2]
        reject_thresholds = [1, 2]

        grid_configs = []
        for acc_dim, acc_avg, imp_min, rej_max in product(
            accept_dims, accept_avgs, improve_mins, reject_thresholds
        ):
            # Skip invalid configs
            if rej_max >= imp_min:
                continue
            if imp_min >= acc_dim:
                continue

            config = ScoringConfig(
                name=f"grid_{acc_dim}_{acc_avg}_{imp_min}_{rej_max}",
                accept_min_dimension=acc_dim,
                accept_min_average=acc_avg,
                improve_min_dimension=imp_min,
                reject_max_dimension=rej_max,
            )
            grid_configs.append(config)

        # Limit grid configs to reasonable number
        configs.extend(grid_configs[:50])

        # Random perturbations around baseline
        baseline = self.BASELINE
        perturbation_configs = []

        for i in range(count - len(configs)):
            # Random perturbations (+/- 1 for integers, +/- 0.25 for floats)
            acc_dim = max(
                1, min(5, baseline.accept_min_dimension + random.choice([-1, 0, 1]))
            )
            acc_avg = max(
                1.0,
                min(
                    5.0,
                    baseline.accept_min_average
                    + random.choice([-0.5, -0.25, 0, 0.25, 0.5]),
                ),
            )
            imp_min = max(
                1, min(4, baseline.improve_min_dimension + random.choice([-1, 0, 1]))
            )
            rej_max = max(
                1, min(3, baseline.reject_max_dimension + random.choice([-1, 0, 1]))
            )

            # Ensure valid config
            if rej_max >= imp_min or imp_min >= acc_dim:
                continue

            config = ScoringConfig(
                name=f"perturb_{i + 1}",
                accept_min_dimension=acc_dim,
                accept_min_average=round(acc_avg, 2),
                improve_min_dimension=imp_min,
                reject_max_dimension=rej_max,
            )
            perturbation_configs.append(config)

        configs.extend(perturbation_configs)

        # Add baseline for comparison
        configs.insert(0, baseline)

        # Remove duplicates
        seen = set()
        unique_configs = []
        for cfg in configs:
            key = (
                cfg.accept_min_dimension,
                cfg.accept_min_average,
                cfg.improve_min_dimension,
                cfg.reject_max_dimension,
            )
            if key not in seen:
                seen.add(key)
                unique_configs.append(cfg)

        return unique_configs[:count]

    def evaluate_config(self, config: ScoringConfig) -> Dict:
        """Evaluate a configuration against verdicts

        Metrics:
        - Agreement rate: How often config matches original verdict
        - Quality score: Weighted by verdict type correctness
        - Edge case handling: Performance on borderline cases
        """
        results = {
            "config": config.to_dict(),
            "total_verdicts": len(self.verdicts),
            "agreements": 0,
            "accept_accuracy": 0,
            "improve_accuracy": 0,
            "reject_accuracy": 0,
            "accept_count": 0,
            "improve_count": 0,
            "reject_count": 0,
            "false_accepts": 0,  # Config says accept, original said reject
            "false_rejects": 0,  # Config says reject, original said accept
            "avg_score_diff": 0.0,
        }

        score_diffs = []

        for v in self.verdicts:
            original = v["verdict"]
            scores = v["scores"]
            predicted, pred_avg = config.get_verdict(scores)
            orig_avg = v.get("average", sum(scores.values()) / len(scores))

            # Track score difference
            score_diffs.append(abs(pred_avg - orig_avg))

            # Track counts
            if predicted == "accept":
                results["accept_count"] += 1
            elif predicted == "improve":
                results["improve_count"] += 1
            else:
                results["reject_count"] += 1

            # Agreement tracking
            if predicted == original:
                results["agreements"] += 1
                if original == "accept":
                    results["accept_accuracy"] += 1
                elif original == "improve":
                    results["improve_accuracy"] += 1
                else:
                    results["reject_accuracy"] += 1
            else:
                # Costly mistakes
                if predicted == "accept" and original == "reject":
                    results["false_accepts"] += 1
                elif predicted == "reject" and original == "accept":
                    results["false_rejects"] += 1

        # Normalize accuracies
        verdict_counts = {"accept": 0, "improve": 0, "reject": 0}
        for v in self.verdicts:
            verdict_counts[v["verdict"]] += 1

        results["accept_accuracy"] = results["accept_accuracy"] / max(
            1, verdict_counts["accept"]
        )
        results["improve_accuracy"] = results["improve_accuracy"] / max(
            1, verdict_counts["improve"]
        )
        results["reject_accuracy"] = results["reject_accuracy"] / max(
            1, verdict_counts["reject"]
        )

        results["agreement_rate"] = results["agreements"] / len(self.verdicts)
        results["avg_score_diff"] = sum(score_diffs) / len(score_diffs)

        # Quality score (lower is better): penalize false accepts heavily
        quality_score = (
            results["agreement_rate"] * 100
            - results["false_accepts"] * 50  # Heavy penalty for false accepts
            - results["false_rejects"] * 20  # Moderate penalty for false rejects
            - results["avg_score_diff"] * 10  # Small penalty for average discrepancies
        )
        results["quality_score"] = round(quality_score, 2)

        return results

    def find_optimal(self, count: int = 100) -> List[Dict]:
        """Generate variations and return top configurations"""
        configs = self.generate_variations(count)
        evaluations = []

        for cfg in configs:
            result = self.evaluate_config(cfg)
            evaluations.append(result)

        # Sort by quality score descending
        evaluations.sort(key=lambda x: x["quality_score"], reverse=True)

        return evaluations[:3]

    def calculate_improvement(self, optimal: Dict, baseline: Dict) -> Dict:
        """Calculate improvement delta"""
        return {
            "quality_score_delta": optimal["quality_score"] - baseline["quality_score"],
            "agreement_rate_delta": optimal["agreement_rate"]
            - baseline["agreement_rate"],
            "false_accepts_delta": optimal["false_accepts"] - baseline["false_accepts"],
            "false_rejects_delta": optimal["false_rejects"] - baseline["false_rejects"],
            "is_significant": (optimal["quality_score"] - baseline["quality_score"])
            > 5.0,
        }


def main():
    """Main entry point"""
    import sys

    # Default paths
    verdicts_path = (
        Path.home() / ".claude" / "skills" / "automation" / "judge" / "verdicts.jsonl"
    )

    if not verdicts_path.exists():
        print(f"Error: Verdicts file not found: {verdicts_path}", file=sys.stderr)
        sys.exit(1)

    print(f"Loading verdicts from: {verdicts_path}")

    optimizer = ConfigOptimizer(verdicts_path)
    print(f"Loaded {len(optimizer.verdicts)} verdicts")
    print()

    # Evaluate baseline first
    print("=== BASELINE CONFIGURATION ===")
    baseline_result = optimizer.evaluate_config(optimizer.BASELINE)
    print(f"Config: {optimizer.BASELINE.to_dict()}")
    print(f"Quality Score: {baseline_result['quality_score']}")
    print(f"Agreement Rate: {baseline_result['agreement_rate']:.2%}")
    print(
        f"Verdict Distribution: accept={baseline_result['accept_count']}, "
        f"improve={baseline_result['improve_count']}, reject={baseline_result['reject_count']}"
    )
    print(
        f"False Accepts: {baseline_result['false_accepts']}, False Rejects: {baseline_result['false_rejects']}"
    )
    print()

    # Find optimal configurations
    print("=== GENERATING 100 CONFIGURATION VARIATIONS ===")
    print("Generating and evaluating configurations...")
    print()

    top_configs = optimizer.find_optimal(100)

    print("=== TOP 3 CANDIDATE CONFIGURATIONS ===")
    print()

    for i, result in enumerate(top_configs, 1):
        print(f"--- Candidate #{i}: {result['config']['name']} ---")
        print(f"Thresholds:")
        print(f"  accept_min_dimension: {result['config']['accept_min_dimension']}")
        print(f"  accept_min_average: {result['config']['accept_min_average']}")
        print(f"  improve_min_dimension: {result['config']['improve_min_dimension']}")
        print(f"  reject_max_dimension: {result['config']['reject_max_dimension']}")
        print(f"Metrics:")
        print(f"  Quality Score: {result['quality_score']}")
        print(f"  Agreement Rate: {result['agreement_rate']:.2%}")
        print(
            f"  False Accepts: {result['false_accepts']}, False Rejects: {result['false_rejects']}"
        )

        if i == 1:
            improvement = optimizer.calculate_improvement(result, baseline_result)
            print(f"Improvement vs Baseline:")
            print(f"  Quality Score Delta: +{improvement['quality_score_delta']:.2f}")
            print(f"  Significant (>5%): {improvement['is_significant']}")
        print()

    # Write results to JSON
    output_path = Path(
        "/home/alex/terraphim-ai/automation/judge-optimizer/results.json"
    )
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(output_path, "w") as f:
        json.dump(
            {
                "baseline": baseline_result,
                "top_candidates": top_configs,
                "timestamp": __import__("datetime")
                .datetime.now(__import__("datetime").timezone.utc)
                .isoformat(),
            },
            f,
            indent=2,
        )

    print(f"Full results written to: {output_path}")

    # Return top candidate for PR creation decision
    top = top_configs[0]
    improvement = optimizer.calculate_improvement(top, baseline_result)

    if improvement["is_significant"]:
        print("\n*** IMPROVEMENT >5% DETECTED ***")
        print("Recommended action: Create draft PR to update scoring thresholds")
        print(f"Proposed config: {top['config']}")
        return 0
    else:
        print("\n*** No significant improvement detected (<=5%) ***")
        print("Current baseline configuration is already optimal")
        return 1


if __name__ == "__main__":
    exit(main())
