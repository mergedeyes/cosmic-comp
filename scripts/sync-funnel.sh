#!/usr/bin/env bash
# Rebase the funnel branch onto current upstream/master, then push the
# result to origin (your fork). Bound to VS Code's default build task
# (Ctrl+Shift+B) via .vscode/tasks.json.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

if [[ "$(git branch --show-current)" != "funnel" ]]; then
  echo "Not on funnel (currently on $(git branch --show-current)) — checking it out." >&2
  git checkout funnel
fi

if [[ -n "$(git status --porcelain)" ]]; then
  echo "Working tree is dirty — commit or stash before syncing." >&2
  exit 1
fi

echo "Fetching upstream (pop-os/cosmic-comp)..."
git fetch upstream

echo "Rebasing funnel onto upstream/master..."
if ! git rebase upstream/master; then
  echo
  echo "Rebase stopped on a conflict. Resolve it in VS Code (Source Control ->" >&2
  echo "Merge Changes), then run: git rebase --continue" >&2
  echo "Or bail out entirely with: git rebase --abort" >&2
  echo "Nothing was pushed." >&2
  exit 1
fi

echo
echo "Rebase clean. Pushing funnel to origin (force-with-lease, since the" 
echo "rebase rewrote history)..."
if git push --force-with-lease origin funnel; then
  echo
  echo "Done. funnel is rebased onto upstream/master and pushed to your fork."
else
  echo
  echo "Push was rejected — origin/funnel has commits your local repo hasn't" >&2
  echo "seen (someone/something else pushed to it). Investigate before forcing" >&2
  echo "further: git fetch origin && git log origin/funnel -5" >&2
  exit 1
fi
