#!/usr/bin/env python3
"""Build and directly run the shared Rust executable on a registered iOS device.

Personal inputs come from IOS_DEVICE and optional IOS_PROVISIONING_PROFILE.
Signed bundles, entitlements, and raw acceptance logs belong under target/.
"""
import argparse
import datetime
import fnmatch
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import signal
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
BUNDLE_ID = "dev.rustdemos.galaxy-menu"
BUILD = ROOT / "target" / "ios"
APP = BUILD / "Galaxy Menu.app"


def private_output(args):
    # Tool errors may include device/account identifiers; keep raw output local.
    result = subprocess.run(args, capture_output=True)
    if result.returncode:
        BUILD.mkdir(parents=True, exist_ok=True)
        (BUILD / "last-error.log").write_bytes(result.stdout + result.stderr)
        raise RuntimeError("Native tool failed; inspect target/ios/last-error.log locally.")
    return result.stdout


def device():
    selected = os.environ.get("IOS_DEVICE")
    if not selected:
        raise RuntimeError("Set IOS_DEVICE to your connected device's identifier or name.")
    data = json.loads(private_output(["xcrun", "devicectl", "list", "devices", "--quiet", "--json-output", "-"]))
    for item in data["result"]["devices"]:
        hardware = item.get("hardwareProperties", {})
        properties = item.get("deviceProperties", {})
        if selected in (item.get("identifier"), hardware.get("udid"), properties.get("name")):
            return item["identifier"], hardware["udid"]
    raise RuntimeError("Selected device was not found; check its connection and pairing.")


def signing(udid):
    identities = private_output(["security", "find-identity", "-v", "-p", "codesigning"]).decode()
    hashes = set(re.findall(r"\b[0-9A-F]{40}\b", identities))
    explicit = os.environ.get("IOS_PROVISIONING_PROFILE")
    profiles = [Path(explicit).expanduser()] if explicit else list(
        (Path.home() / "Library/Developer/Xcode/UserData/Provisioning Profiles").glob("*.mobileprovision")
    ) + list((Path.home() / "Library/MobileDevice/Provisioning Profiles").glob("*.mobileprovision"))
    candidates = []
    for path in profiles:
        result = subprocess.run(["security", "cms", "-D", "-i", str(path)], capture_output=True)
        if result.returncode:
            continue
        profile = plistlib.loads(result.stdout)
        entitlements = profile.get("Entitlements", {})
        pattern = entitlements.get("application-identifier", "").split(".", 1)[-1]
        matches = [hashlib.sha1(cert).hexdigest().upper() for cert in profile.get("DeveloperCertificates", [])]
        matches = [fingerprint for fingerprint in matches if fingerprint in hashes]
        if (profile.get("ExpirationDate", datetime.datetime.min) > datetime.datetime.now(datetime.timezone.utc).replace(tzinfo=None)
                and entitlements.get("get-task-allow") is True
                and fnmatch.fnmatchcase(BUNDLE_ID, pattern)
                and profile.get("ProvisionedDevices")
                and (not udid or udid in profile["ProvisionedDevices"])
                and matches):
            candidates.append((profile["ExpirationDate"], len(profile["ProvisionedDevices"]), path, profile, matches[0]))
    if not candidates:
        raise RuntimeError("No valid matching development profile and signing private key. Register the app/device in Xcode, or set IOS_PROVISIONING_PROFILE.")
    _, _, path, profile, identity = max(candidates, key=lambda item: item[:2])
    prefix = profile["Entitlements"]["application-identifier"].split(".", 1)[0]
    entitlements = {
        "application-identifier": f"{prefix}.{BUNDLE_ID}",
        "com.apple.developer.team-identifier": profile["Entitlements"]["com.apple.developer.team-identifier"],
        "get-task-allow": True,
        "keychain-access-groups": [f"{prefix}.{BUNDLE_ID}"],
    }
    return path, identity, entitlements


def build():
    udid = device()[1] if os.environ.get("IOS_DEVICE") else None
    profile, identity, entitlements = signing(udid)
    env = dict(os.environ, IPHONEOS_DEPLOYMENT_TARGET="27.0")
    target = Path(env.get("CARGO_TARGET_DIR", ROOT / "target")).resolve()
    subprocess.run(["cargo", "build", "-p", "galaxy-menu", "--bin", "galaxy-menu", "--release", "--target", "aarch64-apple-ios"], check=True, cwd=ROOT, env=env)
    if APP.exists():
        shutil.rmtree(APP)
    APP.mkdir(parents=True)
    shutil.copy2(target / "aarch64-apple-ios/release/galaxy-menu", APP / "galaxy-menu")
    shutil.copy2(ROOT / "apps/galaxy-menu/ios/Info.plist", APP / "Info.plist")
    shutil.copy2(profile, APP / "embedded.mobileprovision")
    entitlement_path = BUILD / "development-entitlements.plist"
    entitlement_path.write_bytes(plistlib.dumps(entitlements))
    private_output(["codesign", "--force", "--sign", identity, "--entitlements", str(entitlement_path), str(APP)])
    private_output(["codesign", "--verify", "--strict", str(APP)])
    print("Signed release bundle ready: target/ios/Galaxy Menu.app")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["build", "install", "launch", "baseline", "screenshot"])
    args = parser.parse_args()
    BUILD.mkdir(parents=True, exist_ok=True)
    if args.action == "build":
        build()
        return
    identifier, _ = device()
    common = ["--device", identifier]
    if args.action == "install":
        private_output(["xcrun", "devicectl", "device", "install", "app", *common, str(APP), "--quiet"])
        print("Signed release installed.")
    elif args.action in ("launch", "baseline"):
        command = ["xcrun", "devicectl", "device", "process", "launch", *common, "--terminate-existing", "--quiet"]
        if args.action == "baseline":
            command.remove("--quiet")
            command += ["--console", "--environment-variables", '{"GALAXY_FRAME_BASELINE":"1"}']
            log = BUILD / "baseline.log"
            with log.open("wb") as output:
                process = subprocess.Popen([*command, BUNDLE_ID], stdout=output, stderr=subprocess.STDOUT)
                deadline = time.monotonic() + 120
                try:
                    while process.poll() is None:
                        if "GALAXY_COMPLETE" in log.read_text(errors="replace"):
                            break
                        if time.monotonic() > deadline:
                            raise RuntimeError("Baseline timed out; inspect target/ios/baseline.log locally.")
                        time.sleep(0.25)
                finally:
                    if process.poll() is None:
                        # devicectl forwards SIGINT to the console-attached app.
                        process.send_signal(signal.SIGINT)
                    try:
                        process.wait(timeout=15)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait()
            lines = log.read_text(errors="replace").splitlines()
            if "GALAXY_COMPLETE" not in lines:
                raise RuntimeError("Baseline launch/capture failed; inspect target/ios/baseline.log locally.")
            if not all(any(line.startswith(f"GALAXY_SUMMARY {phase} ") for line in lines) for phase in ("warmup", "steady")):
                raise RuntimeError("Baseline did not finish both measurement phases; inspect target/ios/baseline.log locally.")
            for line in lines:
                if line.startswith(("GALAXY_STARTUP", "GALAXY_CONFIG", "GALAXY_SUMMARY")):
                    print(line)
        else:
            private_output([*command, BUNDLE_ID])
            print("Galaxy Menu launched.")
    elif args.action == "screenshot":
        private_output(["xcrun", "devicectl", "device", "capture", "screenshot", *common, "--destination", str(BUILD / "screenshot.png"), "--quiet"])
        print("Screenshot saved: target/ios/screenshot.png")


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, subprocess.CalledProcessError) as error:
        sys.exit(str(error))
