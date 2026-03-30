#!/usr/bin/env bash
#
# Build, bundle, code sign, and optionally notarize legend-cli as a macOS .app.
#
# The .app bundle is required for iCloud Keychain sync — it embeds a provisioning
# profile that grants the keychain-access-groups entitlement.
#
# Usage:
#   ./scripts/bundle.sh                    # build + bundle + sign
#   ./scripts/bundle.sh --skip-build       # bundle + sign (binary already built)
#   ./scripts/bundle.sh --notarize         # build + bundle + sign + notarize
#
# Environment variables:
#   SIGNING_IDENTITY   - codesign identity (default: auto-detect)
#   NOTARIZE           - set to "1" to notarize (or use --notarize flag)
#   APPLE_ID           - Apple ID email (required for notarization)
#   APPLE_TEAM_ID      - Team ID (default: 747VCSDJ25)
#   APPLE_APP_PASSWORD - app-specific password (required for notarization)
#   TARGET             - cargo build target (default: current host)
#   PROFILE            - cargo profile (default: release)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

APPLE_TEAM_ID="${APPLE_TEAM_ID:-747VCSDJ25}"
PROFILE="${PROFILE:-release}"
SKIP_BUILD=false

for arg in "$@"; do
  case "$arg" in
    --skip-build) SKIP_BUILD=true ;;
    --notarize) NOTARIZE=1 ;;
  esac
done

# Determine target and binary path
if [[ -n "${TARGET:-}" ]]; then
  BINARY="target/$TARGET/$PROFILE/legend-cli"
  BUILD_FLAGS="--target $TARGET"
else
  BINARY="target/$PROFILE/legend-cli"
  BUILD_FLAGS=""
fi

APP_BUNDLE="${BINARY%.legend-cli}legend-cli.app"
APP_BUNDLE="${APP_BUNDLE/legend-cli\/legend-cli.app/legend-cli.app}"

# Use the binary's parent directory for the .app
BINARY_DIR="$(dirname "$BINARY")"
APP_BUNDLE="$BINARY_DIR/legend-cli.app"

# --- Build ---
if [[ "$SKIP_BUILD" == false ]]; then
  echo "Building legend-cli ($PROFILE) with keychain support..."
  cargo build --$PROFILE -p legend-cli --features keychain $BUILD_FLAGS
fi

if [[ ! -f "$BINARY" ]]; then
  echo "Error: binary not found at $BINARY"
  exit 1
fi

# --- Assemble .app bundle ---
echo "Assembling .app bundle..."
rm -rf "$APP_BUNDLE"
mkdir -p "$APP_BUNDLE/Contents/MacOS"
cp bundle/Info.plist "$APP_BUNDLE/Contents/"
cp bundle/embedded.provisionprofile "$APP_BUNDLE/Contents/"
cp "$BINARY" "$APP_BUNDLE/Contents/MacOS/"

echo "  $APP_BUNDLE"
echo "  ├── Contents/"
echo "  │   ├── Info.plist"
echo "  │   ├── embedded.provisionprofile"
echo "  │   └── MacOS/legend-cli"

# --- Code sign ---
if [[ -z "${SIGNING_IDENTITY:-}" ]]; then
  SIGNING_IDENTITY=$(security find-identity -v -p codesigning | grep "Developer ID Application" | head -1 | sed 's/.*"\(.*\)".*/\1/' || true)
  if [[ -z "$SIGNING_IDENTITY" ]]; then
    echo "Error: no Developer ID Application certificate found."
    echo "Install one or set SIGNING_IDENTITY explicitly."
    exit 1
  fi
fi
echo ""
echo "Signing with: $SIGNING_IDENTITY"

codesign --sign "$SIGNING_IDENTITY" \
  --options runtime \
  --entitlements entitlements.plist \
  --force \
  --timestamp \
  "$APP_BUNDLE"

echo "Verifying..."
codesign --verify --deep --strict --verbose=2 "$APP_BUNDLE"
echo "Signed successfully."

# --- Notarize (optional) ---
if [[ "${NOTARIZE:-}" == "1" ]]; then
  echo ""
  echo "=== Notarization ==="

  if [[ -z "${APPLE_ID:-}" ]]; then
    echo "Error: APPLE_ID required for notarization"
    exit 1
  fi
  if [[ -z "${APPLE_APP_PASSWORD:-}" ]]; then
    echo "Error: APPLE_APP_PASSWORD required for notarization"
    exit 1
  fi

  ZIP_PATH="$(mktemp -d)/legend-cli.zip"
  echo "Zipping for notarization..."
  ditto -c -k --keepParent "$APP_BUNDLE" "$ZIP_PATH"

  echo "Submitting to Apple (this may take a few minutes)..."
  xcrun notarytool submit "$ZIP_PATH" \
    --apple-id "$APPLE_ID" \
    --team-id "$APPLE_TEAM_ID" \
    --password "$APPLE_APP_PASSWORD" \
    --wait

  echo "Stapling ticket..."
  xcrun stapler staple "$APP_BUNDLE"

  rm -f "$ZIP_PATH"
  echo "Notarization complete."
fi

echo ""
echo "Done. Run with:"
echo "  $APP_BUNDLE/Contents/MacOS/legend-cli"
