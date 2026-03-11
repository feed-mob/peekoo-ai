import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.release import PROJECT_FILES, bump_version, validate_version


class ValidateVersionTests(unittest.TestCase):
    def test_accepts_semver_triplet(self) -> None:
        self.assertEqual(validate_version("1.2.3"), "1.2.3")

    def test_rejects_invalid_semver_triplet(self) -> None:
        with self.assertRaises(ValueError):
            validate_version("1.2")


class BumpVersionTests(unittest.TestCase):
    @patch("scripts.release.subprocess.run")
    def test_updates_all_project_version_files(self, mock_run) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            cargo_toml = root / "Cargo.toml"
            cargo_toml.write_text(
                '[workspace.package]\nversion = "0.1.0"\n',
                encoding="utf-8",
            )

            tauri_conf = root / "apps/desktop-tauri/src-tauri/tauri.conf.json"
            tauri_conf.parent.mkdir(parents=True)
            tauri_conf.write_text(
                json.dumps({"version": "0.1.0"}),
                encoding="utf-8",
            )

            package_json = root / "apps/desktop-ui/package.json"
            package_json.parent.mkdir(parents=True, exist_ok=True)
            package_json.write_text(
                json.dumps({"version": "0.1.0"}),
                encoding="utf-8",
            )

            tauri_cargo = root / "apps/desktop-tauri/src-tauri/Cargo.toml"
            tauri_cargo.write_text(
                '[package]\nversion = "0.1.0"\n',
                encoding="utf-8",
            )

            changed_files = bump_version(root, "1.4.2")

            self.assertEqual(changed_files, PROJECT_FILES + ["Cargo.lock"])
            self.assertIn('version = "1.4.2"', cargo_toml.read_text(encoding="utf-8"))
            self.assertEqual(
                json.loads(tauri_conf.read_text(encoding="utf-8"))["version"],
                "1.4.2",
            )
            self.assertEqual(
                json.loads(package_json.read_text(encoding="utf-8"))["version"],
                "1.4.2",
            )
            self.assertIn('version = "1.4.2"', tauri_cargo.read_text(encoding="utf-8"))
            mock_run.assert_called_once_with(
                ["cargo", "update", "--workspace"],
                cwd=root,
                check=True,
            )


if __name__ == "__main__":
    unittest.main()
