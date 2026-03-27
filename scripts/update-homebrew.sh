#!/usr/bin/env bash
#
# Update the Homebrew tap formula after a new legend-cli release.
#
# Usage:
#   ./scripts/update-homebrew.sh           # auto-detect latest release
#   ./scripts/update-homebrew.sh v0.0.4    # specific version
#   ./scripts/update-homebrew.sh --push    # auto-detect, commit, and push
#
# Environment variables:
#   HOMEBREW_TAP  - path to the tap repo (default: sibling of monorepo)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
MONOREPO_ROOT="$(cd "$PROJECT_DIR/../../.." && pwd)"
REPO="legend-hq/legend-cli"

PUSH=false
VERSION=""
for arg in "$@"; do
  case "$arg" in
    --push) PUSH=true ;;
    v*) VERSION="$arg" ;;
  esac
done

# Find the tap repo
TAP="${HOMEBREW_TAP:-}"
if [[ -z "$TAP" ]]; then
  CANDIDATE="$(dirname "$MONOREPO_ROOT")/homebrew-tap"
  if [[ -d "$CANDIDATE/.git" ]]; then
    TAP="$CANDIDATE"
  fi
fi
if [[ -z "$TAP" ]]; then
  echo "Error: Could not find homebrew-tap repo. Set HOMEBREW_TAP or place it next to the monorepo."
  exit 1
fi

FORMULA="$TAP/Formula/legend-cli.rb"
if [[ ! -f "$FORMULA" ]]; then
  echo "Error: Formula not found at $FORMULA"
  exit 1
fi

# Get latest version from GitHub if not specified
if [[ -z "$VERSION" ]]; then
  VERSION=$(gh release view --repo "$REPO" --json tagName -q .tagName 2>/dev/null || true)
  if [[ -z "$VERSION" ]]; then
    echo "Error: Could not determine latest release. Pass a version or check gh auth."
    exit 1
  fi
fi

# Strip 'v' prefix for the formula version field
SEMVER="${VERSION#v}"
echo "Updating formula to $VERSION ($SEMVER)..."

# Download and hash the macOS artifact
URL="https://github.com/$REPO/releases/download/$VERSION/legend-cli-macos-aarch64.tar.gz"
echo "Fetching SHA256 for $URL ..."
SHA=$(curl -sL "$URL" | shasum -a 256 | awk '{print $1}')

if [[ -z "$SHA" || "$SHA" == "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855" ]]; then
  echo "Error: Failed to download artifact (empty or missing). Is the release published?"
  exit 1
fi

echo "  SHA256: $SHA"

# Update the formula
sed -i '' "s/version \".*\"/version \"$SEMVER\"/" "$FORMULA"
sed -i '' "s/sha256 \".*\"/sha256 \"$SHA\"/" "$FORMULA"

echo ""
echo "Updated $FORMULA:"
grep -E 'version|sha256' "$FORMULA" | head -2

if [[ "$PUSH" == true ]]; then
  echo ""
  cd "$TAP"
  git add Formula/legend-cli.rb
  git commit -m "legend-cli $SEMVER"
  git push
  echo "Pushed to homebrew-tap."
else
  echo ""
  echo "Next steps:"
  echo "  cd $TAP"
  echo "  git add -A && git commit -m 'legend-cli $SEMVER'"
  echo "  git push"
fi
