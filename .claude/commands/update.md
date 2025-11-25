Bump the patch version in Cargo.toml and nix/package.nix, then commit with a changelog.

Steps:
1. Read Cargo.toml and find the current version (e.g., "0.1.0")
2. Increment the patch version by 1 (e.g., "0.1.0" → "0.1.1")
3. Edit Cargo.toml to update the version
4. Edit nix/package.nix to update the version (same new version)
5. Check `git log` to find the previous version tag commit
6. Review changes since last version using `git diff <previous-commit>..HEAD --stat` and recent commit messages
7. Stage all changes with `git add -A`
8. Commit with changelog format:

```
v{new_version}

Changes:
- Summary of change 1
- Summary of change 2
- etc.
```

Example:
```
v0.1.6

Changes:
- Added CLI parity (get, search, delete, pin, count commands)
- Reorganized documentation into docs/ folder
- Added /task slash command for creating tasks
```

Do NOT push to remote. Just make the local commit.
