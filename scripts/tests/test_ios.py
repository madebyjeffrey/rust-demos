"""Exercise the native capture command through its command-line boundary."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "ios.py"


class BaselineCommandTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        self.arguments = self.directory / "launch.json"
        xcrun = self.directory / "xcrun"
        xcrun.write_text('''#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
if sys.argv[1:4] == ["devicectl", "list", "devices"]:
    print(json.dumps({"result": {"devices": [{"identifier": "fixture", "hardwareProperties": {"udid": "fixture"}, "deviceProperties": {"name": "Fixture"}}]}}))
else:
    Path(os.environ["CAPTURE_ARGUMENTS"]).write_text(json.dumps(sys.argv[1:]))
    print(os.environ["CAPTURE_LOG"], flush=True)
''')
        xcrun.chmod(0o755)
        self.environment = dict(
            os.environ,
            PATH=f"{self.directory}{os.pathsep}{os.environ['PATH']}",
            IOS_DEVICE="fixture",
            CAPTURE_ARGUMENTS=str(self.arguments),
            CAPTURE_LOG="\n".join([
                "GALAXY_STARTUP first_update_ms=300.000",
                "GALAXY_CONFIG scene_width=640 scene_height=480 physical_width=1280 physical_height=960",
                "GALAXY_SUMMARY warmup frames=1 mean_ms=16.000",
                "GALAXY_FRAME warmup 0.016000 16.000000",
                "GALAXY_SUMMARY steady frames=1 mean_ms=16.000",
                "GALAXY_FRAME steady 0.266000 16.000000",
                "GALAXY_EVENTS warmup touch_started=0 touch_moved=0 window_resized=0",
                "GALAXY_EVENTS steady touch_started=1 touch_moved=2 window_resized=1",
                "GALAXY_COMPLETE",
            ]),
        )

    def command(self, *arguments):
        return subprocess.run(
            [sys.executable, str(SCRIPT), "baseline", *arguments],
            env=self.environment, capture_output=True, text=True, timeout=10,
        )

    def test_named_capture_passes_configuration_to_app_and_preserves_raw_log(self):
        output = self.directory / "orbit.log"
        result = self.command("--scenario", "orbit", "--warmup-seconds", "0.25",
                              "--steady-seconds", "0.5", "--output", str(output))
        self.assertEqual(result.returncode, 0, result.stderr)
        launched = json.loads(self.arguments.read_text())
        environment = json.loads(launched[launched.index("--environment-variables") + 1])
        self.assertEqual(environment, {
            "GALAXY_FRAME_BASELINE": "1", "GALAXY_BASELINE_SCENARIO": "orbit",
            "GALAXY_BASELINE_WARMUP_SECONDS": "0.25", "GALAXY_BASELINE_STEADY_SECONDS": "0.5",
        })
        self.assertIn("GALAXY_FRAME steady 0.266000 16.000000", output.read_text())
        self.assertIn("GALAXY_EVENTS steady touch_started=1", result.stdout)

    def test_completed_marker_without_raw_steady_frames_is_rejected(self):
        self.environment["CAPTURE_LOG"] = self.environment["CAPTURE_LOG"].replace(
            "GALAXY_FRAME steady 0.266000 16.000000\n", "")
        result = self.command("--output", str(self.directory / "partial.log"))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("raw frame samples", result.stderr)


if __name__ == "__main__":
    unittest.main()
