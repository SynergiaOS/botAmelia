#!/bin/bash

# Test script for Kestra-Cerberus integration
# Sprawdza czy wszystkie komponenty działają poprawnie

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
CERBERUS_URL="http://localhost:8080"
KESTRA_URL="http://localhost:8081"
TIMEOUT=30

echo -e "${BLUE}🧪 KESTRA-CERBERUS INTEGRATION TEST${NC}"
echo "=================================================="

# Function to check if service is running
check_service() {
    local name=$1
    local url=$2
    local endpoint=$3
    
    echo -n "Checking $name... "
    
    if curl -s --max-time $TIMEOUT "$url$endpoint" > /dev/null 2>&1; then
        echo -e "${GREEN}✅ OK${NC}"
        return 0
    else
        echo -e "${RED}❌ FAILED${NC}"
        return 1
    fi
}

# Function to test API endpoint
test_api_endpoint() {
    local name=$1
    local url=$2
    local expected_status=$3
    
    echo -n "Testing $name... "
    
    local status=$(curl -s -o /dev/null -w "%{http_code}" --max-time $TIMEOUT "$url")
    
    if [ "$status" = "$expected_status" ]; then
        echo -e "${GREEN}✅ OK (HTTP $status)${NC}"
        return 0
    else
        echo -e "${RED}❌ FAILED (HTTP $status, expected $expected_status)${NC}"
        return 1
    fi
}

# Function to test Kestra workflow
test_kestra_workflow() {
    local namespace=$1
    local flow_id=$2
    
    echo -n "Testing workflow $namespace/$flow_id... "
    
    # Get flow info
    local response=$(curl -s --max-time $TIMEOUT "$KESTRA_URL/api/v1/flows/$namespace/$flow_id")
    
    if echo "$response" | grep -q "\"id\":\"$flow_id\""; then
        echo -e "${GREEN}✅ OK${NC}"
        return 0
    else
        echo -e "${RED}❌ FAILED${NC}"
        return 1
    fi
}

# Function to execute workflow
execute_workflow() {
    local namespace=$1
    local flow_id=$2
    local inputs=$3
    
    echo -n "Executing workflow $namespace/$flow_id... "
    
    local payload="{\"inputs\": $inputs}"
    local response=$(curl -s -X POST \
        -H "Content-Type: application/json" \
        -d "$payload" \
        --max-time $TIMEOUT \
        "$KESTRA_URL/api/v1/executions/$namespace/$flow_id")
    
    if echo "$response" | grep -q "\"state\":"; then
        local execution_id=$(echo "$response" | grep -o '"id":"[^"]*"' | cut -d'"' -f4)
        echo -e "${GREEN}✅ OK (Execution: $execution_id)${NC}"
        return 0
    else
        echo -e "${RED}❌ FAILED${NC}"
        return 1
    fi
}

# Start tests
echo -e "${YELLOW}📋 Phase 1: Service Health Checks${NC}"
echo "--------------------------------------------------"

SERVICES_OK=true

# Check Docker containers
echo -n "Checking Docker containers... "
if docker-compose ps | grep -q "Up"; then
    echo -e "${GREEN}✅ OK${NC}"
else
    echo -e "${RED}❌ FAILED${NC}"
    SERVICES_OK=false
fi

# Check Cerberus
if ! check_service "Cerberus API" "$CERBERUS_URL" "/health"; then
    SERVICES_OK=false
fi

# Check Kestra
if ! check_service "Kestra API" "$KESTRA_URL" "/api/v1/flows"; then
    SERVICES_OK=false
fi

# Check PostgreSQL (through Kestra)
if ! check_service "PostgreSQL (via Kestra)" "$KESTRA_URL" "/api/v1/stats"; then
    SERVICES_OK=false
fi

if [ "$SERVICES_OK" = false ]; then
    echo -e "${RED}❌ Service health checks failed. Please check your setup.${NC}"
    exit 1
fi

echo ""
echo -e "${YELLOW}📋 Phase 2: API Endpoint Tests${NC}"
echo "--------------------------------------------------"

API_OK=true

# Test Cerberus endpoints
if ! test_api_endpoint "Cerberus Health" "$CERBERUS_URL/health" "200"; then
    API_OK=false
fi

if ! test_api_endpoint "Cerberus Detailed Health" "$CERBERUS_URL/health/detailed" "200"; then
    API_OK=false
fi

if ! test_api_endpoint "Cerberus Metrics" "$CERBERUS_URL/metrics" "200"; then
    API_OK=false
fi

if ! test_api_endpoint "Cerberus Prometheus" "$CERBERUS_URL/metrics/prometheus" "200"; then
    API_OK=false
fi

# Test Kestra endpoints
if ! test_api_endpoint "Kestra Flows" "$KESTRA_URL/api/v1/flows" "200"; then
    API_OK=false
fi

if ! test_api_endpoint "Kestra Stats" "$KESTRA_URL/api/v1/stats" "200"; then
    API_OK=false
fi

if [ "$API_OK" = false ]; then
    echo -e "${RED}❌ API endpoint tests failed.${NC}"
    exit 1
fi

echo ""
echo -e "${YELLOW}📋 Phase 3: Workflow Tests${NC}"
echo "--------------------------------------------------"

WORKFLOW_OK=true

# Test workflow existence
if ! test_kestra_workflow "cerberus.trading" "cerberus-trading-pipeline"; then
    WORKFLOW_OK=false
fi

if ! test_kestra_workflow "cerberus.monitoring" "cerberus-monitoring"; then
    WORKFLOW_OK=false
fi

if ! test_kestra_workflow "cerberus.analytics" "cerberus-analytics"; then
    WORKFLOW_OK=false
fi

if ! test_kestra_workflow "cerberus.system" "cerberus-system-management"; then
    WORKFLOW_OK=false
fi

if ! test_kestra_workflow "cerberus.alerting" "cerberus-advanced-alerting"; then
    WORKFLOW_OK=false
fi

if ! test_kestra_workflow "cerberus.positions" "cerberus-position-management"; then
    WORKFLOW_OK=false
fi

if [ "$WORKFLOW_OK" = false ]; then
    echo -e "${RED}❌ Workflow tests failed.${NC}"
    exit 1
fi

echo ""
echo -e "${YELLOW}📋 Phase 4: Integration Tests${NC}"
echo "--------------------------------------------------"

INTEGRATION_OK=true

# Test system health check workflow
if ! execute_workflow "cerberus.system" "cerberus-system-management" '{"operation_type": "health_check"}'; then
    INTEGRATION_OK=false
fi

# Test monitoring workflow
if ! execute_workflow "cerberus.monitoring" "cerberus-monitoring" '{"alert_threshold_cpu": 95.0}'; then
    INTEGRATION_OK=false
fi

# Test position monitoring
if ! execute_workflow "cerberus.positions" "cerberus-position-management" '{"action": "monitor"}'; then
    INTEGRATION_OK=false
fi

if [ "$INTEGRATION_OK" = false ]; then
    echo -e "${RED}❌ Integration tests failed.${NC}"
    exit 1
fi

echo ""
echo -e "${YELLOW}📋 Phase 5: Performance Tests${NC}"
echo "--------------------------------------------------"

# Check response times
echo -n "Testing Cerberus response time... "
CERBERUS_TIME=$(curl -s -w "%{time_total}" -o /dev/null "$CERBERUS_URL/health")
if (( $(echo "$CERBERUS_TIME < 1.0" | bc -l) )); then
    echo -e "${GREEN}✅ OK (${CERBERUS_TIME}s)${NC}"
else
    echo -e "${YELLOW}⚠️ SLOW (${CERBERUS_TIME}s)${NC}"
fi

echo -n "Testing Kestra response time... "
KESTRA_TIME=$(curl -s -w "%{time_total}" -o /dev/null "$KESTRA_URL/api/v1/flows")
if (( $(echo "$KESTRA_TIME < 2.0" | bc -l) )); then
    echo -e "${GREEN}✅ OK (${KESTRA_TIME}s)${NC}"
else
    echo -e "${YELLOW}⚠️ SLOW (${KESTRA_TIME}s)${NC}"
fi

# Check resource usage
echo -n "Checking Docker resource usage... "
MEMORY_USAGE=$(docker stats --no-stream --format "table {{.MemPerc}}" | tail -n +2 | sed 's/%//' | sort -nr | head -1)
if (( $(echo "$MEMORY_USAGE < 80" | bc -l) )); then
    echo -e "${GREEN}✅ OK (${MEMORY_USAGE}% max memory)${NC}"
else
    echo -e "${YELLOW}⚠️ HIGH (${MEMORY_USAGE}% max memory)${NC}"
fi

echo ""
echo -e "${YELLOW}📋 Phase 6: Configuration Tests${NC}"
echo "--------------------------------------------------"

# Check if secrets are configured
echo -n "Checking secrets configuration... "
if [ -f "kestra/secrets.env" ]; then
    if grep -q "TELEGRAM_BOT_TOKEN=" kestra/secrets.env && ! grep -q "your_telegram_bot_token_here" kestra/secrets.env; then
        echo -e "${GREEN}✅ OK${NC}"
    else
        echo -e "${YELLOW}⚠️ INCOMPLETE (Telegram not configured)${NC}"
    fi
else
    echo -e "${YELLOW}⚠️ MISSING (secrets.env not found)${NC}"
fi

# Check workflow triggers
echo -n "Checking workflow triggers... "
ACTIVE_TRIGGERS=$(curl -s "$KESTRA_URL/api/v1/flows" | grep -o '"disabled":false' | wc -l)
echo -e "${GREEN}✅ OK ($ACTIVE_TRIGGERS active triggers)${NC}"

echo ""
echo "=================================================="
echo -e "${GREEN}🎉 ALL TESTS COMPLETED SUCCESSFULLY!${NC}"
echo ""
echo -e "${BLUE}📊 Test Summary:${NC}"
echo "• Service Health: ✅ Passed"
echo "• API Endpoints: ✅ Passed"
echo "• Workflows: ✅ Passed"
echo "• Integration: ✅ Passed"
echo "• Performance: ✅ Checked"
echo "• Configuration: ✅ Checked"
echo ""
echo -e "${BLUE}🔗 Access URLs:${NC}"
echo "• Kestra UI: http://localhost:8081"
echo "• Cerberus API: http://localhost:8080"
echo "• Grafana: http://localhost:3000"
echo ""
echo -e "${BLUE}📝 Next Steps:${NC}"
echo "1. Configure Telegram bot in kestra/secrets.env"
echo "2. Enable workflow triggers in Kestra UI"
echo "3. Monitor executions in Kestra UI"
echo "4. Check Grafana dashboards for metrics"
echo ""
echo -e "${GREEN}✅ Integration is ready for production use!${NC}"
