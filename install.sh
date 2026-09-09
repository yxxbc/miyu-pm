#!/usr/bin/env bash
#
# Online installer for miyu-pm
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/yxxbc/miyu-pm/main/install.sh | sh
#
# Env overrides:
#   MIYU_PM_REPO         GitHub repo, default yxxbc/miyu-pm
#   MIYU_PM_VERSION      release tag without v, default latest
#   MIYU_PM_INSTALL_DIR  install directory, default $HOME/.local/bin

set -euo pipefail

REPO="${MIYU_PM_REPO:-yxxbc/miyu-pm}"
INSTALL_DIR="${MIYU_PM_INSTALL_DIR:-${HOME}/.local/bin}"

echo "==> miyu-pm online installer"

# Resolve latest version if not pinned
if [[ -z "${MIYU_PM_VERSION:-}" || "${MIYU_PM_VERSION}" == "latest" ]]; then
    API="https://api.github.com/repos/${REPO}/releases/latest"
    VERSION="$(curl -fsSL "${API}" \
        | grep -o '"tag_name": *"[^"]*"' \
        | head -1 \
        | sed -E 's/.*"([^"]+)"$/\1/' \
        | sed 's/^v//')"
    if [[ -z "${VERSION}" ]]; then
        echo "error: could not resolve latest release from ${REPO}" >&2
        exit 1
    fi
else
    VERSION="${MIYU_PM_VERSION}"
fi

# Detect platform
case "$(uname -s)" in
    Linux) OS="linux" ;;
    Darwin) OS="macos" ;;
    *)
        echo "error: unsupported OS: $(uname -s)" >&2
        exit 1
        ;;
esac

case "$(uname -m)" in
    x86_64|amd64) ARCH="x86_64" ;;
    arm64|aarch64) ARCH="aarch64" ;;
    *)
        echo "error: unsupported arch: $(uname -m)" >&2
        exit 1
        ;;
esac

ASSET="miyu-pm-${VERSION}-${OS}-${ARCH}.tar.gz"
URL="https://github.com/${REPO}/releases/download/v${VERSION}/${ASSET}"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

echo "==> version : ${VERSION}"
echo "==> os/arch: ${OS}/${ARCH}"
echo "==> download: ${URL}"

curl -fsSL --max-time 120 -o "${TMP_DIR}/${ASSET}" "${URL}"
tar -xzf "${TMP_DIR}/${ASSET}" -C "${TMP_DIR}"

mkdir -p "${INSTALL_DIR}"
install -m 0755 "${TMP_DIR}/miyu-pm" "${INSTALL_DIR}/miyu-pm"

SHARE_DIR="$(dirname "${INSTALL_DIR}")/share/miyu-pm"
if [[ -d "${TMP_DIR}/tui/bin" ]]; then
    TUI_SRC="${TMP_DIR}/tui"
elif [[ -d "${TMP_DIR}/tui/tui/bin" ]]; then
    # Compatibility with release assets that accidentally nested tui/tui.
    TUI_SRC="${TMP_DIR}/tui/tui"
else
    TUI_SRC=""
fi
if [[ -n "${TUI_SRC}" ]]; then
    mkdir -p "${SHARE_DIR}"
    cp -R "${TUI_SRC}" "${SHARE_DIR}/"
    echo "==> installed TUI to ${SHARE_DIR}/tui"
fi

echo "==> installed to ${INSTALL_DIR}/miyu-pm"
echo "==> run: ${INSTALL_DIR}/miyu-pm --help"
if ! echo "${PATH}" | grep -q "${INSTALL_DIR}"; then
    echo "hint: add ${INSTALL_DIR} to your PATH"
fi
