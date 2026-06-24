#!/bin/bash

set -e

cargo build --release
./target/release/llm_wiki