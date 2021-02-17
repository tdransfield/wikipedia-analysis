#!/bin/bash

# This script builds the tool, runs the xml parser to generate the intermediate
# datasets then uses the intermediate datasets to generate various outputs

WIKIPEDIA_DUMP_PATH=./enwiki-20171103-pages-articles-multistream.xml
IGNORE_DIR=./ignored-articles
DATASETS_DIR=./datasets
RESULTS_DIR=./results
EXECUTABLE=./target/release/wikipedia-analysis

STEP_GROUPS_LENGTH=1000

mkdir -p $DATASETS_DIR
mkdir -p $RESULTS_DIR

# Build tool
cargo build --release

set -x # Enable echo

# Generate datasets
$EXECUTABLE parse --output $DATASETS_DIR/incoming-ignored.tsv --ignore-dir $IGNORE_DIR $WIKIPEDIA_DUMP_PATH
$EXECUTABLE parse --output $DATASETS_DIR/incoming-no-ignore.tsv $WIKIPEDIA_DUMP_PATH
$EXECUTABLE parse --output $DATASETS_DIR/outgoing-ignored.tsv -r --ignore-dir $IGNORE_DIR $WIKIPEDIA_DUMP_PATH
$EXECUTABLE parse --output $DATASETS_DIR/outgoing-no-ignore.tsv -r $WIKIPEDIA_DUMP_PATH

# Generate link historgrams
$EXECUTABLE analyze --input $DATASETS_DIR/outgoing-ignored.tsv --output $RESULTS_DIR/outgoing-histogram-ignored.tsv link-histogram
$EXECUTABLE analyze --input $DATASETS_DIR/incoming-ignored.tsv --output $RESULTS_DIR/incoming-histogram-ignored.tsv link-histogram
$EXECUTABLE analyze --input $DATASETS_DIR/outgoing-no-ignore.tsv --output $RESULTS_DIR/outgoing-histogram-no-ignore.tsv link-histogram
$EXECUTABLE analyze --input $DATASETS_DIR/incoming-no-ignore.tsv --output $RESULTS_DIR/incoming-histogram-no-ignore.tsv link-histogram

# Generate most linked
$EXECUTABLE analyze --input $DATASETS_DIR/outgoing-ignored.tsv --output $RESULTS_DIR/outgoing-most-linked-ignored.tsv most-linked
$EXECUTABLE analyze --input $DATASETS_DIR/incoming-ignored.tsv --output $RESULTS_DIR/incoming-most-linked-ignored.tsv most-linked
$EXECUTABLE analyze --input $DATASETS_DIR/outgoing-no-ignore.tsv --output $RESULTS_DIR/outgoing-most-linked-no-ignore.tsv most-linked
$EXECUTABLE analyze --input $DATASETS_DIR/incoming-no-ignore.tsv --output $RESULTS_DIR/incoming-most-linked-no-ignore.tsv most-linked

# Generate step groups for top $STEP_GROUPS_LENGTH most linked articles
$EXECUTABLE analyze --input $DATASETS_DIR/outgoing-ignored.tsv --output $RESULTS_DIR/outgoing-step-groups-most-linked-ignored.tsv step-groups --use-most-linked $STEP_GROUPS_LENGTH
$EXECUTABLE analyze --input $DATASETS_DIR/incoming-ignored.tsv --output $RESULTS_DIR/incoming-step-groups-most-linked-ignored.tsv step-groups --use-most-linked $STEP_GROUPS_LENGTH
$EXECUTABLE analyze --input $DATASETS_DIR/outgoing-no-ignore.tsv --output $RESULTS_DIR/outgoing-step-groups-most-linked-no-ignore.tsv step-groups --use-most-linked $STEP_GROUPS_LENGTH
$EXECUTABLE analyze --input $DATASETS_DIR/incoming-no-ignore.tsv --output $RESULTS_DIR/incoming-step-groups-most-linked-no-ignore.tsv step-groups --use-most-linked $STEP_GROUPS_LENGTH

# Generate step groups for $STEP_GROUPS_LENGTH randomly selected articles
$EXECUTABLE analyze --input $DATASETS_DIR/outgoing-ignored.tsv --output $RESULTS_DIR/outgoing-step-groups-random-ignored.tsv step-groups --use-random $STEP_GROUPS_LENGTH

# Use the same random articles for the other datasets. Note we are choosing the random articles from an ignored corpus
# as all articles in the ignored set should be in the non-ignored set.
cat $RESULTS_DIR/outgoing-step-groups-random-ignored.tsv | cut -f1 | tail -n +2 > $RESULTS_DIR/random_article_names.txt

$EXECUTABLE analyze --input $DATASETS_DIR/incoming-ignored.tsv --output $RESULTS_DIR/incoming-step-groups-random-ignored.tsv step-groups --roots-file $RESULTS_DIR/random_article_names.txt
$EXECUTABLE analyze --input $DATASETS_DIR/outgoing-no-ignore.tsv --output $RESULTS_DIR/outgoing-step-groups-random-no-ignore.tsv step-groups --roots-file $RESULTS_DIR/random_article_names.txt
$EXECUTABLE analyze --input $DATASETS_DIR/incoming-no-ignore.tsv --output $RESULTS_DIR/incoming-step-groups-random-no-ignore.tsv step-groups --roots-file $RESULTS_DIR/random_article_names.txt

set +x # Disable echo