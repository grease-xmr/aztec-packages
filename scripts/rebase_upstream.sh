#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

# Usage: run from your repo root on the branch you want to update (e.g. master)
# Example: git checkout master && ./rebase-upstream.sh

# detect current branch
LOCAL_BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "$LOCAL_BRANCH" = "HEAD" ]; then
  echo "Error: detached HEAD. Checkout the branch you want to update." >&2
  exit 1
fi

# require clean working tree
if ! git diff-index --quiet HEAD --; then
  echo "Error: working tree has uncommitted changes. Commit or stash them before running." >&2
  exit 1
fi

# ensure upstream remote exists
if ! git remote get-url upstream >/dev/null 2>&1; then
  echo "Error: remote 'upstream' not found." >&2
  exit 1
fi

# fetch upstream
git fetch upstream --prune

# detect upstream default branch (falls back to 'master' if detection fails)
UPSTREAM_BRANCH=$(git remote show upstream | awk '/HEAD branch/ {print $NF}')
UPSTREAM_BRANCH=${UPSTREAM_BRANCH:-master}


echo "Local branch: $LOCAL_BRANCH"
echo "Upstream default branch: $UPSTREAM_BRANCH"

# create a safety backup of the current branch
BACKUP="backup/${LOCAL_BRANCH}-before-rebase-$(date +%Y%m%d%H%M%S)"
git branch -f "$BACKUP" "$LOCAL_BRANCH"
echo "Created backup branch: $BACKUP"

# rebase local branch onto upstream
echo "Rebasing $LOCAL_BRANCH onto upstream/$UPSTREAM_BRANCH..."
git rebase "upstream/$UPSTREAM_BRANCH"

# push the rebased branch to origin (force with lease to be safe)
echo "Pushing $LOCAL_BRANCH to origin with --force-with-lease..."
git push --force-with-lease origin "$LOCAL_BRANCH"

# --- Cleanup: remove backup and prune upstream remote-tracking branches ---
# Delete the local backup branch we created
if git show-ref --verify --quiet "refs/heads/$BACKUP"; then
  git branch -D "$BACKUP"
  echo "Deleted local backup branch: $BACKUP"
fi

# Prune stale remote-tracking refs for upstream (removes refs under refs/remotes/upstream/* that no longer exist on remote)
git remote prune upstream
echo "Pruned stale remote-tracking refs for 'upstream'."

echo "Done. $LOCAL_BRANCH is rebased onto upstream/$UPSTREAM_BRANCH and pushed to origin."
