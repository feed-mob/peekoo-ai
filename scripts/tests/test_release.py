import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import call, patch

from scripts.release import (
    PROJECT_FILES,
    apply_release_label_if_available,
    build_release_pr_body,
    bump_version,
    create_release_branch,
    create_release_commit,
    create_release_pr,
    ensure_gh_authentication,
    parse_gh_json,
    push_release_branch,
    validate_version,
)


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


class ReleaseGitFlowTests(unittest.TestCase):
    @patch("scripts.release.subprocess.run")
    def test_create_release_branch_uses_versioned_branch_name(self, mock_run) -> None:
        create_release_branch(Path("/tmp/project"), "1.4.2")

        mock_run.assert_called_once_with(
            ["git", "switch", "-c", "release/1.4.2"],
            cwd=Path("/tmp/project"),
            check=True,
        )

    @patch("scripts.release.subprocess.run")
    def test_create_release_commit_does_not_create_a_tag(self, mock_run) -> None:
        create_release_commit(Path("/tmp/project"), "1.4.2")

        self.assertEqual(
            mock_run.call_args_list,
            [
                call(
                    ["git", "add", *PROJECT_FILES, "Cargo.lock"],
                    cwd=Path("/tmp/project"),
                    check=True,
                ),
                call(
                    ["git", "commit", "-S", "-m", "chore(release): 1.4.2"],
                    cwd=Path("/tmp/project"),
                    check=True,
                ),
            ],
        )

    @patch("scripts.release.subprocess.run")
    def test_push_release_branch_sets_upstream(self, mock_run) -> None:
        push_release_branch(Path("/tmp/project"), "1.4.2")

        mock_run.assert_called_once_with(
            ["git", "push", "-u", "origin", "release/1.4.2"],
            cwd=Path("/tmp/project"),
            check=True,
        )


class ReleasePullRequestTests(unittest.TestCase):
    def test_build_release_pr_body_matches_template(self) -> None:
        self.assertEqual(
            build_release_pr_body("1.4.2"),
            "## What changed\n\n"
            "- Bump release version to 1.4.2\n\n"
            "## Release notes\n\n"
            "- [ ] Add at least one release-note label: `feature`, `fix`, `docs`, `test`, `chore`, `ci`, or `refactor`\n"
            "- [ ] Use `skip-changelog` if this PR should be excluded from generated release notes\n\n"
            "## Verification\n\n"
            "- [ ] Tests pass locally\n",
        )

    @patch("scripts.release.apply_release_label_if_available")
    @patch("scripts.release.subprocess.run")
    def test_create_release_pr_uses_gh_with_release_branch(
        self, mock_run, mock_apply_label
    ) -> None:
        create_release_pr(Path("/tmp/project"), "1.4.2")

        self.assertEqual(
            mock_run.call_args_list,
            [
                call(
                    ["gh", "auth", "status"],
                    cwd=Path("/tmp/project"),
                    check=True,
                    capture_output=True,
                    text=True,
                ),
                call(
                    [
                        "gh",
                        "pr",
                        "create",
                        "--base",
                        "master",
                        "--head",
                        "release/1.4.2",
                        "--title",
                        "chore(release): 1.4.2",
                        "--body",
                        build_release_pr_body("1.4.2"),
                    ],
                    cwd=Path("/tmp/project"),
                    check=True,
                ),
            ],
        )
        mock_apply_label.assert_called_once_with(Path("/tmp/project"), "1.4.2")

    def test_parse_gh_json_reads_command_output(self) -> None:
        with patch("scripts.release.subprocess.run") as mock_run:
            mock_run.return_value.stdout = '[{"name": "chore"}]'

            result = parse_gh_json(
                Path("/tmp/project"), "label", "list", "--json", "name"
            )

        self.assertEqual(result, [{"name": "chore"}])
        mock_run.assert_called_once_with(
            ["gh", "label", "list", "--json", "name"],
            cwd=Path("/tmp/project"),
            check=True,
            capture_output=True,
            text=True,
        )

    def test_apply_release_label_if_available_adds_chore_label(self) -> None:
        with patch(
            "scripts.release.parse_gh_json", return_value=[{"name": "chore"}]
        ) as mock_parse:
            with patch("scripts.release.subprocess.run") as mock_run:
                apply_release_label_if_available(Path("/tmp/project"), "1.4.2")

        mock_parse.assert_called_once_with(
            Path("/tmp/project"), "label", "list", "--json", "name", "--limit", "200"
        )
        mock_run.assert_called_once_with(
            [
                "gh",
                "pr",
                "edit",
                "release/1.4.2",
                "--add-label",
                "chore",
            ],
            cwd=Path("/tmp/project"),
            check=True,
        )

    def test_apply_release_label_if_available_skips_missing_label(self) -> None:
        with patch(
            "scripts.release.parse_gh_json", return_value=[{"name": "docs"}]
        ) as mock_parse:
            with patch("scripts.release.subprocess.run") as mock_run:
                apply_release_label_if_available(Path("/tmp/project"), "1.4.2")

        mock_parse.assert_called_once_with(
            Path("/tmp/project"), "label", "list", "--json", "name", "--limit", "200"
        )
        mock_run.assert_not_called()

    @patch("scripts.release.subprocess.run")
    def test_ensure_gh_authentication_checks_auth_status(self, mock_run) -> None:
        ensure_gh_authentication(Path("/tmp/project"))

        mock_run.assert_called_once_with(
            ["gh", "auth", "status"],
            cwd=Path("/tmp/project"),
            check=True,
            capture_output=True,
            text=True,
        )

    @patch("scripts.release.subprocess.run")
    def test_ensure_gh_authentication_raises_clear_error_when_not_logged_in(
        self, mock_run
    ) -> None:
        mock_run.side_effect = subprocess.CalledProcessError(
            1,
            ["gh", "auth", "status"],
            stderr="not logged into any GitHub hosts",
        )

        with self.assertRaisesRegex(SystemExit, "GitHub CLI is not authenticated"):
            ensure_gh_authentication(Path("/tmp/project"))


if __name__ == "__main__":
    unittest.main()
