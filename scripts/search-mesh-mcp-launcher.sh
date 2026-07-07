#!/usr/bin/env bash
set -euo pipefail

REPO="jamestkelly/search-mesh"
BIN_NAME="search-mesh-mcp"
INSTALL_DIR="${CLAUDE_PLUGIN_ROOT}/bin"
BIN_PATH="${INSTALL_DIR}/${BIN_NAME}"

if [ -x "${BIN_PATH}" ]; then
  exec "${BIN_PATH}" "$@"
fi

log() {
  printf '%s\n' "$*" >&2
}

fail() {
  log "$*"
  exit 1
}

manual_install_hint="Install manually instead: cargo install ${BIN_NAME}"

os="$(uname -s)"
arch="$(uname -m)"

case "${os}" in
  Darwin)
    case "${arch}" in
      arm64) target="aarch64-apple-darwin" ;;
      x86_64) target="x86_64-apple-darwin" ;;
      *) fail "${BIN_NAME}: unsupported macOS architecture '${arch}'. ${manual_install_hint}" ;;
    esac
    ;;
  Linux)
    case "${arch}" in
      x86_64) target="x86_64-unknown-linux-gnu" ;;
      *) fail "${BIN_NAME}: unsupported Linux architecture '${arch}'. ${manual_install_hint}" ;;
    esac
    ;;
  *)
    fail "${BIN_NAME}: no prebuilt binary for '${os}'. ${manual_install_hint}"
    ;;
esac

command -v curl >/dev/null 2>&1 || fail "${BIN_NAME}: 'curl' is required to auto-install. ${manual_install_hint}"

log "${BIN_NAME}: binary not found, installing for ${target}..."

tag="$( (curl -fsSL "https://api.github.com/repos/${REPO}/releases" |
  grep -o "\"tag_name\": *\"${BIN_NAME}-v[^\"]*\"" |
  head -n1 |
  sed -E 's/.*"([^"]+)"$/\1/') || true)"

[ -n "${tag}" ] || fail "${BIN_NAME}: could not resolve latest release. ${manual_install_hint}"

archive="${BIN_NAME}-${target}.tar.gz"
checksum_file="${BIN_NAME}-${target}.sha256"
base_url="https://github.com/${REPO}/releases/download/${tag}"

workdir="$(mktemp -d)"
trap 'rm -rf "${workdir}"' EXIT

curl -fsSL -o "${workdir}/${archive}" "${base_url}/${archive}" ||
  fail "${BIN_NAME}: failed to download ${archive} from release ${tag}."
curl -fsSL -o "${workdir}/${checksum_file}" "${base_url}/${checksum_file}" ||
  fail "${BIN_NAME}: failed to download checksum for ${archive} from release ${tag}."

(
  cd "${workdir}"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum -c "${checksum_file}" >&2
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 -c "${checksum_file}" >&2
  else
    exit 2
  fi
) || fail "${BIN_NAME}: checksum verification failed for ${archive}."

tar -xzf "${workdir}/${archive}" -C "${workdir}"
[ -x "${workdir}/${BIN_NAME}" ] || fail "${BIN_NAME}: extracted archive did not contain expected binary."

mkdir -p "${INSTALL_DIR}"
tmp_bin="${INSTALL_DIR}/.${BIN_NAME}.tmp.$$"
cp "${workdir}/${BIN_NAME}" "${tmp_bin}"
chmod +x "${tmp_bin}"
mv "${tmp_bin}" "${BIN_PATH}"

exec "${BIN_PATH}" "$@"
