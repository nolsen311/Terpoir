// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

// The English Tesseract model (android/ocr/README.md). On Play it is an
// on-demand asset pack, fetched with the `:ocr` module on first use; the
// foss app bundles the same file (app/build.gradle.kts).

plugins {
    alias(libs.plugins.android.asset.pack)
}

assetPack {
    packName.set("ocr_model")
    dynamicDelivery {
        deliveryType.set("on-demand")
    }
}
