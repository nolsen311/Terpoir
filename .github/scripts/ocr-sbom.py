#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Usage: ocr-sbom.py AAR > terpoir-ocr-native.cdx.json
#
# A CycloneDX SBOM for the OCR native libraries built from source
# (android/ocr-native/): the built AAR as the main component, and each
# pinned source in android/ocr-native/sources.txt (version, download URL,
# SHA-256) as a component. Gradle's SBOM can't describe them, because the
# release build takes the AAR as a file. Run from the repository root.

import hashlib
import json
import sys

# SPDX identifiers of the sources' licences (texts in
# android/app/src/main/assets/licenses/). A new source needs one here.
LICENSES = {
    "tesseract4android": "Apache-2.0",
    "tesseract": "Apache-2.0",
    "leptonica": "BSD-2-Clause",
    "libpng": "libpng-2.0",
    "libjpeg": "IJG",
    "androidx-annotation": "Apache-2.0",
}


def main() -> None:
    aar = sys.argv[1]
    with open(aar, "rb") as f:
        aar_hash = hashlib.sha256(f.read()).hexdigest()
    components = []
    version = None
    with open("android/ocr-native/sources.txt", encoding="utf-8") as f:
        for line in f:
            if not line.strip() or line.startswith("#"):
                continue
            name, ver, sha256, url = line.split()
            if name == "tesseract4android":
                version = ver
            components.append(
                {
                    "type": "library",
                    "bom-ref": f"{name}@{ver}",
                    "name": name,
                    "version": ver,
                    "hashes": [{"alg": "SHA-256", "content": sha256}],
                    "licenses": [{"license": {"id": LICENSES[name]}}],
                    "externalReferences": [{"type": "distribution", "url": url}],
                }
            )
    bom = {
        "bomFormat": "CycloneDX",
        "specVersion": "1.6",
        "version": 1,
        "metadata": {
            "component": {
                "type": "library",
                "bom-ref": "terpoir-ocr-native",
                "name": "tesseract4android (built from source)",
                "version": version,
                "description": "Tesseract4Android rebuilt from pinned sources by android/ocr-native/build.sh",
                "hashes": [{"alg": "SHA-256", "content": aar_hash}],
            }
        },
        "components": components,
        "dependencies": [
            {"ref": "terpoir-ocr-native", "dependsOn": [c["bom-ref"] for c in components]}
        ],
    }
    json.dump(bom, sys.stdout, indent=2)
    sys.stdout.write("\n")


main()
