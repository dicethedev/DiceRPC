cat > scripts/test_all.sh << 'EOF'
#!/bin/bash

set -e

echo "╔══════════════════════════════════════╗"
echo "║     DiceRPC Complete Test Suite      ║"
echo "╚══════════════════════════════════════╝"
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Test 1: Unit tests
echo "Running unit tests..."
cargo test --lib
echo -e "${GREEN} Unit tests passed${NC}"
echo ""

# Test 2: TCP integration tests
echo " Running TCP integration tests..."
cargo test --test tcp_integration --features tcp
echo -e "${GREEN} TCP tests passed${NC}"
echo ""

# Test 3: HTTP integration tests
echo "Running HTTP integration tests..."
cargo test --test http_integration --features http
echo -e "${GREEN} HTTP tests passed${NC}"
echo ""

# Test 4: Build all features
echo "Building with all features..."
cargo build --features full
echo -e "${GREEN} Build successful${NC}"
echo ""

# Test 5: Run examples (compile only)
echo "Checking examples compile..."
cargo build --example tcp_basic
cargo build --example http_basic --features http
echo -e "${GREEN} Examples compile${NC}"
echo ""

echo "╔══════════════════════════════════════╗"
echo "║     All Tests Passed!                ║"
echo "╚══════════════════════════════════════╝"
EOF

chmod +x scripts/test_all.sh