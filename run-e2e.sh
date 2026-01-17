#!/bin/bash
# End-to-End Verification for Gemini Rust Infrastructure
set -e

echo "🚀 Starting Full E2E Verification..."

# 1. Build Kernel & Python Bridge
echo "Building kernel..."
cd .gemini/rust/rsk
maturin develop --features python > /dev/null 2>&1

# 2. Run E2E Test Suite
echo "Executing E2E Suite..."
/home/matthew/.gemini/rust/rsk/.venv/bin/python tests/test_e2e_runtime.py

echo "💎 Gemini Rust Infrastructure: VERIFIED"
