# Release procedure

Maintainer steps for one release. Example run: notes/runs/261113-release-0126.md.
CI does the version bump, the `release: X.Y.Z` commit, the tag, and the build (`.github/workflows/release-dispatch.yml`, `release.yml`).

1. Run the release: Actions → release-dispatch → Run workflow on master, or `gh workflow run release-dispatch.yml -f version=X.Y.Z`.
   It runs `scripts/bump-version.sh X.Y.Z`, commits `release: X.Y.Z`, tags `vX.Y.Z`, pushes, then calls release.yml.
   Wait for both runs with the bounded poll in `.claude/agents/haiku-runner.md` ("Waiting"). Never `gh run watch`: it hangs silently after a job fails. Stop at the first failed job and read `gh run view <id> --log-failed | tail -n 80`.
2. Locally, `git pull --tags` so the new tag exists.
3. Usage stats for the cycle (this project only, local transcripts; not run in CI):
   `python3 -I scripts/release-usage.py --from <prev tag> --to vX.Y.Z --out notes/usage/X.Y.Z.md`
   Before the tag exists, `--to` takes an ISO time instead of a tag.
4. Commit it: `git add notes/usage/X.Y.Z.md && git commit -m "release: usage stats X.Y.Z"`. Push with the next master push.
5. Put the summary in the release report: cycle totals, 5h block count, peak 5h block (from the file header and tables).

Never commit `.claude/settings.json`, `.run/*`, or `notes/research/*` with a release.
