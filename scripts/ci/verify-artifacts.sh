#!/usr/bin/env bash

set -euo pipefail

ARTIFACT_ROOT="${1:-dist}"

if [[ ! -d "${ARTIFACT_ROOT}" ]]; then
  echo "artifact directory not found: ${ARTIFACT_ROOT}" >&2
  exit 1
fi

if ! find "${ARTIFACT_ROOT}" -mindepth 1 -maxdepth 1 -type d -print -quit | grep -q .; then
  echo "no artifacts found in: ${ARTIFACT_ROOT}" >&2
  exit 1
fi

fail=0

for artifact_dir in "${ARTIFACT_ROOT}"/*; do
  [[ -d "${artifact_dir}" ]] || continue

  primary_found=0

  if find "${artifact_dir}" -type d -name "*.app" -print -quit | grep -q .; then
    primary_found=1
  fi

  for pattern in "*.dmg" "*.app.zip" "*.msi" "*.exe" "*.AppImage" "*.deb" "*.rpm"; do
    if find "${artifact_dir}" -type f -name "${pattern}" -print -quit | grep -q .; then
      primary_found=1
    fi
  done

  for pattern in "boxy-cli-tui-*.tar.gz" "boxy-cli-tui-*.zip"; do
    if find "${artifact_dir}" -type f -name "${pattern}" -print -quit | grep -q .; then
      primary_found=1
    fi
  done

  if [[ "${primary_found}" -ne 1 ]]; then
    echo "no bundle artifacts found in: ${artifact_dir}" >&2
    fail=1
    continue
  fi

  if ! find "${artifact_dir}" -type f -size +0c -print -quit | grep -q .; then
    echo "artifact files are empty in: ${artifact_dir}" >&2
    fail=1
  fi
done

exit "${fail}"
