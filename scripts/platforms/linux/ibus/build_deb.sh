#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
PACKAGE_NAME="khmerime"
VERSION="${KHMERIME_PACKAGE_VERSION:-$(awk -F '"' '/^version = / { print $2; exit }' "${ROOT_DIR}/Cargo.toml")}"
ARCH="${KHMERIME_DEB_ARCH:-amd64}"
DIST_DIR="${ROOT_DIR}/dist/linux"
PACKAGE_ROOT="${DIST_DIR}/deb-root"
DEBIAN_DIR="${PACKAGE_ROOT}/DEBIAN"
LIBEXEC_DIR="${PACKAGE_ROOT}/usr/libexec/khmerime"
COMPONENT_DIR="${PACKAGE_ROOT}/usr/share/ibus/component"
DOC_DIR="${PACKAGE_ROOT}/usr/share/doc/khmerime"
OUT_DEB="${DIST_DIR}/${PACKAGE_NAME}_${VERSION}_${ARCH}.deb"
CONTROL_TEMPLATE="${ROOT_DIR}/packaging/linux/deb/control.in"
POSTINST_SRC="${ROOT_DIR}/packaging/linux/deb/postinst"
POSTRM_SRC="${ROOT_DIR}/packaging/linux/deb/postrm"
ENGINE_SCRIPT_SRC="${ROOT_DIR}/adapters/linux-ibus/python/khmerime_ibus_engine.py"
ENGINE_HELPER_SRC="${ROOT_DIR}/adapters/linux-ibus/python/ibus_segment_preview.py"
BRIDGE_SRC="${ROOT_DIR}/target/release/khmerime_ibus_bridge"

if ! command -v dpkg-deb >/dev/null 2>&1; then
  echo "dpkg-deb is required to build the Debian package" >&2
  exit 2
fi

if [[ ! -f "${CONTROL_TEMPLATE}" ]]; then
  echo "Missing Debian control template: ${CONTROL_TEMPLATE}" >&2
  exit 2
fi

if [[ ! -f "${POSTINST_SRC}" || ! -f "${POSTRM_SRC}" ]]; then
  echo "Missing Debian maintainer scripts under packaging/linux/deb" >&2
  exit 2
fi

if [[ ! -f "${ENGINE_SCRIPT_SRC}" || ! -f "${ENGINE_HELPER_SRC}" ]]; then
  echo "Missing Linux IBus Python adapter files" >&2
  exit 2
fi

echo "[khmerime] building khmerime_ibus_bridge (release)..."
cargo build --release --bin khmerime_ibus_bridge >/dev/null

rm -rf "${PACKAGE_ROOT}"
mkdir -p "${DEBIAN_DIR}" "${LIBEXEC_DIR}" "${COMPONENT_DIR}" "${DOC_DIR}" "${DIST_DIR}"

sed \
  -e "s/@VERSION@/${VERSION}/g" \
  -e "s/@ARCH@/${ARCH}/g" \
  "${CONTROL_TEMPLATE}" > "${DEBIAN_DIR}/control"
install -m 0755 "${POSTINST_SRC}" "${DEBIAN_DIR}/postinst"
install -m 0755 "${POSTRM_SRC}" "${DEBIAN_DIR}/postrm"

install -m 0755 "${BRIDGE_SRC}" "${LIBEXEC_DIR}/khmerime-ibus-bridge"
install -m 0755 "${ENGINE_SCRIPT_SRC}" "${LIBEXEC_DIR}/khmerime-ibus-engine"
install -m 0644 "${ENGINE_HELPER_SRC}" "${LIBEXEC_DIR}/ibus_segment_preview.py"

cat > "${COMPONENT_DIR}/khmerime.xml" <<XML
<component>
    <name>org.freedesktop.IBus.KhmerIME</name>
    <description>KhmerIME input method engine</description>
    <version>${VERSION}</version>
    <license>MIT</license>
    <author>KhmerIME contributors</author>
    <homepage>https://github.com/darhnoel/khmerime</homepage>
    <textdomain>khmerime</textdomain>
    <exec>/usr/libexec/khmerime/khmerime-ibus-engine --ibus --bridge-path /usr/libexec/khmerime/khmerime-ibus-bridge</exec>
    <engines>
        <engine>
            <name>khmerime</name>
            <longname>KhmerIME</longname>
            <description>Khmer romanization IME powered by KhmerIME</description>
            <language>km</language>
            <license>MIT</license>
            <author>KhmerIME contributors</author>
            <icon></icon>
            <layout>us</layout>
            <symbol>ខ</symbol>
            <rank>80</rank>
        </engine>
    </engines>
</component>
XML

if [[ -f "${ROOT_DIR}/README.md" ]]; then
  install -m 0644 "${ROOT_DIR}/README.md" "${DOC_DIR}/README.md"
fi

find "${PACKAGE_ROOT}" -type d -exec chmod 0755 {} +
dpkg-deb --build --root-owner-group "${PACKAGE_ROOT}" "${OUT_DEB}"

echo "[khmerime] Debian package written to: ${OUT_DEB}"
echo "[khmerime] Inspect with: dpkg-deb -I ${OUT_DEB} && dpkg-deb -c ${OUT_DEB}"
