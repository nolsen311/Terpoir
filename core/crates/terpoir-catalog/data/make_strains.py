#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Usage: make_strains.py KUSHY_CSV > strains-kushy.tsv
#
# Builds the offline strain catalog from Kushy's MIT-licensed dataset
# (https://github.com/kushyapp/cannabis-dataset, commit
# 8b807c3dfc0aaa1e217fba4924b26ded0774d5aa,
# Dataset/Strains/strains-kushy_api.2017-11-14.csv).
#
# Only the factual columns are kept (maintainer, 2026-10-21): name, type,
# breeder and parents. Descriptions, effects, ailments, flavors, terpenes and
# the cannabinoid figures are left out (docs/research/strain-lookups.md).
#
# Output: a header line, then one strain per line, tab-separated:
#   name, type ("indica", "sativa" or empty), breeder (or empty), parents
#   (names separated by " | ", or empty). Sorted by the compared name.
#
# Cleaning:
# - Names and breeders mis-decoded as Windows-1252 ("Godâ€™s") are repaired.
# - "Hybrid" is dropped: 92% of rows carry it, so it reads as the dataset's
#   default rather than a fact. Values that aren't a type ("Organi",
#   "Indoor") are dropped too.
# - "Unknown Breeder" and "NULL" are empty.
# - Parents are Kushy row IDs, resolved to names; unknown IDs are dropped.
# - Rows with the same name (ignoring case, spacing and punctuation, as the
#   app compares names) are merged: the first non-empty value of each column
#   wins, and the first row's spelling of the name is kept.

import csv
import sys

TYPES = {"indica": "indica", "sativa": "sativa"}
EMPTY = {"", "null", "unknown breeder", "0"}


def repair(text):
    text = text.strip()
    if "Ã" in text or "â€" in text:
        try:
            text = text.encode("cp1252").decode("utf-8")
        except (UnicodeEncodeError, UnicodeDecodeError):
            pass
    return " ".join(text.replace("\t", " ").split())


def value(text):
    text = repair(text)
    return "" if text.lower() in EMPTY else text


def key(name):
    return " ".join("".join(c if c.isalnum() else " " for c in name).lower().split())


def main(path):
    with open(path, encoding="utf-8", newline="") as f:
        rows = list(csv.DictReader(f))
    names = {r["id"].strip(): repair(r["name"]) for r in rows}
    merged = {}
    for r in rows:
        name = repair(r["name"])
        if not name:
            continue
        parents = []
        for pid in r["crosses"].split(","):
            parent = names.get(pid.strip(), "")
            if parent and parent.lower() != name.lower() and parent not in parents:
                parents.append(parent)
        entry = [
            name,
            TYPES.get(r["type"].strip().lower(), ""),
            value(r["breeder"]),
            " | ".join(parents),
        ]
        k = key(name)
        if not k:
            continue
        if k in merged:
            merged[k] = [old or new for old, new in zip(merged[k], entry)]
        else:
            merged[k] = entry
    out = sys.stdout
    out.write("name\ttype\tbreeder\tparents\n")
    for k in sorted(merged):
        out.write("\t".join(merged[k]) + "\n")


if __name__ == "__main__":
    main(sys.argv[1])
