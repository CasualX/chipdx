#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/../.." && pwd)"

cd "$repo_root"

if [[ -t 1 ]]; then
	color_reset=$'\033[0m'
	color_bold=$'\033[1m'
	color_dim=$'\033[2m'
	color_blue=$'\033[34m'
	color_yellow=$'\033[33m'
	color_red=$'\033[31m'
	color_green=$'\033[32m'
else
	color_reset=""
	color_bold=""
	color_dim=""
	color_blue=""
	color_yellow=""
	color_red=""
	color_green=""
fi

progress() {
	printf "\n%b%s%b %b%s%b\n" "$color_blue" "==" "$color_reset" "$color_bold" "$1" "$color_reset"
}

is_ci() {
	[[ -n "${CI:-}" || -n "${GITHUB_ACTIONS:-}" ]]
}

pause_for_ack() {
	local prompt="$1"
	if is_ci; then
		printf "\n%b•%b %s %b[auto-confirmed in CI]%b\n" "$color_yellow" "$color_reset" "$prompt" "$color_dim" "$color_reset"
		return
	fi
	printf "\n%b•%b %s\n" "$color_yellow" "$color_reset" "$prompt"
	printf "  %b[Press Enter to continue or Ctrl-C to abort]%b" "$color_dim" "$color_reset"
	read -r _
}

ensure_git_identity() {
	local repo="$1"
	if ! git -C "$repo" config user.name >/dev/null; then
		git -C "$repo" config user.name "github-actions[bot]"
	fi
	if ! git -C "$repo" config user.email >/dev/null; then
		git -C "$repo" config user.email "41898282+github-actions[bot]@users.noreply.github.com"
	fi
}

warn_if_dirty_worktree() {
	if ! git diff-index --quiet HEAD --; then
		printf "%bWarning:%b git working tree has staged or unstaged changes.\n" "$color_yellow" "$color_reset"
		pause_for_ack "Continue with a dirty worktree?"
	fi
}

progress "Checking repository state"
warn_if_dirty_worktree

progress "Building release chipwasm artifact"
cargo build --release -p chipwasm --target=wasm32-unknown-unknown

pause_for_ack "About to recreate the gh-pages branch and worktree."

progress "Verifying repository state before gh-pages setup"
if git worktree list --porcelain | grep -Fqx "worktree $repo_root/gh-pages"; then
	progress "Removing existing gh-pages worktree"
	git worktree remove --force "$repo_root/gh-pages"
fi

if [[ -e "$repo_root/gh-pages" ]]; then
	printf "%bError:%b %s exists after worktree cleanup.\n" "$color_red" "$color_reset" "$repo_root/gh-pages" >&2
	printf "%bError:%b Refusing to continue because that path should be managed only by git worktree.\n" "$color_red" "$color_reset" >&2
	exit 1
fi

progress "Recreating gh-pages branch and worktree"
git branch -D gh-pages >/dev/null 2>&1 || true
git worktree add --orphan "$repo_root/gh-pages"

progress "Copying gh-pages contents"
cp -R code/chiphtml/public/. "$repo_root/gh-pages/"
cp docs/social-preview.png "$repo_root/gh-pages/social-preview.png"
cp target/wasm32-unknown-unknown/release/chipwasm.wasm "$repo_root/gh-pages/chipwasm.wasm"

progress "Creating gh-pages commit"
ensure_git_identity "$repo_root/gh-pages"
git -C "$repo_root/gh-pages" add .
git -C "$repo_root/gh-pages" commit --allow-empty -m "Initial commit"

progress "Removing gh-pages worktree"
git worktree remove --force "$repo_root/gh-pages"

pause_for_ack "About to force-push to origin/gh-pages."

progress "Force-pushing gh-pages"
git push -f origin gh-pages

progress "Done. Wait for GitHub Pages to finish publishing."
printf "%b✓%b Game URL: https://casualhacks.net/chipdx/\n" "$color_green" "$color_reset"
