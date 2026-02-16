#!/bin/bash
# Nakama Database Connection Diagnostic Script
# Usage: ./check-connection.sh

echo "=== Nakama Database Connection Check ==="
echo ""

# Check if Docker containers are running
echo "1. Container Status:"
docker ps --filter "name=tower-" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
echo ""

# Check PostgreSQL is accessible
echo "2. PostgreSQL Health:"
docker exec tower-postgres pg_isready -U nakama
if [ $? -eq 0 ]; then
    echo "✅ PostgreSQL is ready"
else
    echo "❌ PostgreSQL is not ready"
fi
echo ""

# Test direct PostgreSQL connection with correct credentials
echo "3. Direct PostgreSQL Connection (nakama user):"
docker exec tower-postgres psql -U nakama -d nakama -c "SELECT version();" 2>&1 | head -2
echo ""

# Check Nakama logs for database connection
echo "4. Nakama Database Connection Logs:"
docker logs tower-nakama 2>&1 | grep -i "database\|postgres\|cockroach" | tail -10
echo ""

# Check Nakama environment variables
echo "5. Nakama Database Environment:"
docker exec tower-nakama env | grep -i "database\|nakama" | sort
echo ""

# Check if Nakama is trying to connect to CockroachDB (port 26257) or PostgreSQL (port 5432)
echo "6. Network Connections:"
docker exec tower-nakama netstat -tuln 2>/dev/null | grep -E "5432|26257" || echo "netstat not available in container"
echo ""

# Try Nakama health check
echo "7. Nakama Health Check:"
curl -s http://localhost:7350/healthcheck || echo "❌ Nakama HTTP endpoint not responding"
echo ""

# Summary
echo ""
echo "=== Diagnosis Summary ==="
echo "If you see errors about 'user root':"
echo "  - Nakama is still defaulting to CockroachDB settings"
echo "  - Try: docker-compose down -v && docker-compose up -d"
echo ""
echo "If PostgreSQL connection fails:"
echo "  - Check password in docker-compose.yml matches local.yml"
echo "  - Verify: docker exec tower-postgres psql -U nakama -c '\\l'"
echo ""
echo "If Nakama won't start:"
echo "  - Check logs: docker-compose logs nakama"
echo "  - Check config syntax: docker exec tower-nakama cat /nakama/data/local.yml"
