#!/bin/bash

TARGET_DIR="${CARGO_TARGET_DIR:-"target"}"

# echo $TARGET_DIR

{ time $TARGET_DIR/release/brc >res.txt; } 2> eres.txt
{ time $TARGET_DIR/release/brc >res.txt; } 2>> eres.txt
{ time $TARGET_DIR/release/brc >res.txt; } 2>> eres.txt
{ time $TARGET_DIR/release/brc >res.txt; } 2>> eres.txt
{ time $TARGET_DIR/release/brc >res.txt; } 2>> eres.txt

grep real eres.txt | awk '{print $2}' | sort | tail -4 | head -3
