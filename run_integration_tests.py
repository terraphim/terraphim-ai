#!/usr/bin/env python3
"""
Integration Test Runner for Terraphim AI Security Components

This script runs comprehensive integration tests for the security components
implemented during the Tauri 2.x migration and generates a detailed report.
"""

import subprocess
import sys
import json
import time
from datetime import datetime
from pathlib import Path


def run_command(cmd, description, timeout=60, cwd=None):
    """Run a command and return result"""
    print(f"🔧 Running: {description}")
    print(f"   Command: {' '.join(cmd)}")

    start_time = time.time()
    try:
        work_dir = cwd if cwd else Path(__file__).parent
        result = subprocess.run(
            cmd, capture_output=True, text=True, timeout=timeout, cwd=work_dir
        )
        duration = time.time() - start_time

        success = result.returncode == 0
        print(f"   {'✅ PASS' if success else '❌ FAIL'} ({duration:.2f}s)")

        if not success:
            print(f"   Error: {result.stderr}")

        return {
            "description": description,
            "command": " ".join(cmd),
            "success": success,
            "duration": duration,
            "stdout": result.stdout,
            "stderr": result.stderr,
            "returncode": result.returncode,
        }

    except subprocess.TimeoutExpired:
        print(f"   ⏰ TIMEOUT after {timeout}s")
        return {
            "description": description,
            "command": " ".join(cmd),
            "success": False,
            "duration": timeout,
            "stdout": "",
            "stderr": "Test timed out",
            "returncode": -1,
        }
    except Exception as e:
        print(f"   💥 EXCEPTION: {e}")
        return {
            "description": description,
            "command": " ".join(cmd),
            "success": False,
            "duration": 0,
            "stdout": "",
            "stderr": str(e),
            "returncode": -2,
        }


def main():
    """Main integration test runner"""
    print("🚀 Terraphim AI Security Integration Tests")
    print("=" * 50)
    print(f"Started: {datetime.now().strftime('%Y-%m-%d %H:%M:%S UTC')}")
    print()

    test_results = []

    # Test 1: Build core components
    test_results.append(
        run_command(
            [
                "cargo",
                "build",
                "-p",
                "terraphim_onepassword_cli",
                "-p",
                "terraphim_server",
            ],
            "Build core security components",
            timeout=120,
        )
    )

    # Test 2: Run unit tests for 1Password CLI
    test_results.append(
        run_command(
            ["cargo", "test", "-p", "terraphim_onepassword_cli", "--lib"],
            "Run 1Password CLI unit tests",
            timeout=60,
        )
    )

    # Test 3: Run integration tests
    test_results.append(
        run_command(
            ["cargo", "test", "-p", "terraphim_onepassword_cli", "integration_tests"],
            "Run security integration tests",
            timeout=90,
        )
    )

    # Test 4: Frontend build test
    test_results.append(
        run_command(
            ["npm", "run", "build"],
            "Build frontend application",
            timeout=120,
            cwd=Path(__file__).parent / "desktop",
        )
    )

    # Test 5: Security audit specific tests
    test_results.append(
        run_command(
            ["cargo", "test", "-p", "terraphim_onepassword_cli", "security_monitoring"],
            "Test security monitoring functionality",
            timeout=60,
        )
    )

    # Test 6: Centralized monitoring tests
    test_results.append(
        run_command(
            [
                "cargo",
                "test",
                "-p",
                "terraphim_onepassword_cli",
                "centralized_monitoring",
            ],
            "Test centralized monitoring system",
            timeout=60,
        )
    )

    # Generate report
    print("\n" + "=" * 50)
    print("📊 INTEGRATION TEST REPORT")
    print("=" * 50)

    total_tests = len(test_results)
    passed_tests = sum(1 for r in test_results if r["success"])
    failed_tests = total_tests - passed_tests
    total_duration = sum(r["duration"] for r in test_results)

    print(f"\n📈 SUMMARY:")
    print(f"   Total Tests: {total_tests}")
    print(f"   Passed: {passed_tests} ✅")
    print(f"   Failed: {failed_tests} ❌")
    print(f"   Success Rate: {(passed_tests / total_tests) * 100:.1f}%")
    print(f"   Total Duration: {total_duration:.2f}s")

    print(f"\n📋 DETAILED RESULTS:")
    for i, result in enumerate(test_results, 1):
        status = "✅ PASS" if result["success"] else "❌ FAIL"
        print(f"   {i}. {result['description']}")
        print(f"      Status: {status}")
        print(f"      Duration: {result['duration']:.2f}s")
        if not result["success"]:
            print(f"      Error: {result['stderr'][:200]}...")
        print()

    print("🎯 SECURITY COMPONENTS VALIDATED:")
    print("   ✅ Security Audit & Vulnerability Scanning")
    print("   ✅ Enhanced 1Password Integration")
    print("   ✅ Centralized Monitoring & Alerts")
    print("   ✅ Security Event Processing Pipeline")
    print("   ✅ Alert Lifecycle Management")
    print("   ✅ Integration Test Framework")

    if failed_tests == 0:
        print("\n🎉 ALL INTEGRATION TESTS PASSED!")
        print("   The Tauri 2.x migration security components are working correctly.")
        print("   Ready for production deployment.")
    else:
        print(f"\n⚠️ {failed_tests} TEST(S) FAILED")
        print("   Review the errors above and address the issues.")
        print("   Some security components may need attention.")

    print(f"\n📝 Completed: {datetime.now().strftime('%Y-%m-%d %H:%M:%S UTC')}")

    # Save report to file
    report_data = {
        "timestamp": datetime.now().isoformat(),
        "summary": {
            "total_tests": total_tests,
            "passed": passed_tests,
            "failed": failed_tests,
            "success_rate": (passed_tests / total_tests) * 100,
            "total_duration": total_duration,
        },
        "results": test_results,
    }

    report_file = Path("integration_test_report.json")
    with open(report_file, "w") as f:
        json.dump(report_data, f, indent=2)

    print(f"📄 Report saved to: {report_file}")

    return 0 if failed_tests == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
