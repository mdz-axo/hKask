#!/bin/bash
# hKask Monitoring Stack Deployment Script

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

log() {
    echo -e "[$(date '+%Y-%m-%d %H:%M:%S')] $1"
}

log_success() {
    log "${GREEN}✓ $1${NC}"
}

log_warning() {
    log "${YELLOW}⚠ $1${NC}"
}

log_error() {
    log "${RED}✗ $1${NC}"
}

# Check prerequisites
check_prereqs() {
    log "Checking prerequisites..."
    
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed"
        exit 1
    fi
    
    if ! command -v docker-compose &> /dev/null; then
        log_error "Docker Compose is not installed"
        exit 1
    fi
    
    log_success "Prerequisites check passed"
}

# Create necessary directories
setup_directories() {
    log "Setting up directories..."
    
    mkdir -p grafana/datasources
    mkdir -p grafana/dashboards
    mkdir -p alerts
    mkdir -p prometheus_data
    mkdir -p grafana_data
    mkdir -p alertmanager_data
    
    log_success "Directories created"
}

# Start the monitoring stack
start_stack() {
    log "Starting monitoring stack..."
    
    docker-compose up -d
    
    log_success "Services started"
}

# Wait for services to be healthy
wait_for_services() {
    log "Waiting for services to be healthy..."
    
    # Wait for Prometheus
    log "Waiting for Prometheus..."
    for i in {1..30}; do
        if curl -s http://localhost:9090/-/healthy > /dev/null 2>&1; then
            log_success "Prometheus is healthy"
            break
        fi
        if [ $i -eq 30 ]; then
            log_error "Prometheus failed to become healthy"
            docker-compose logs prometheus
            exit 1
        fi
        sleep 2
    done
    
    # Wait for Grafana
    log "Waiting for Grafana..."
    for i in {1..30}; do
        if curl -s http://localhost:3000/api/health > /dev/null 2>&1; then
            log_success "Grafana is healthy"
            break
        fi
        if [ $i -eq 30 ]; then
            log_error "Grafana failed to become healthy"
            docker-compose logs grafana
            exit 1
        fi
        sleep 2
    done
    
    # Wait for Alertmanager
    log "Waiting for Alertmanager..."
    for i in {1..30}; do
        if curl -s http://localhost:9093/-/healthy > /dev/null 2>&1; then
            log_success "Alertmanager is healthy"
            break
        fi
        if [ $i -eq 30 ]; then
            log_error "Alertmanager failed to become healthy"
            docker-compose logs alertmanager
            exit 1
        fi
        sleep 2
    done
    
    log_success "All services are healthy"
}

# Display access information
show_access_info() {
    echo ""
    echo "=========================================="
    log_success "Monitoring stack is running!"
    echo "=========================================="
    echo ""
    echo "Access URLs:"
    echo "  - Prometheus:   http://localhost:9090"
    echo "  - Grafana:      http://localhost:3000 (admin/admin)"
    echo "  - Alertmanager: http://localhost:9093"
    echo "  - Node Exporter: http://localhost:9100/metrics"
    echo ""
    echo "Next steps:"
    echo "  1. Open Grafana and configure dashboards"
    echo "  2. Start hKask application with metrics endpoint"
    echo "  3. Configure Okapi instances in prometheus.yml"
    echo ""
    echo "Useful commands:"
    echo "  - View logs:     docker-compose logs -f"
    echo "  - Stop stack:    docker-compose down"
    echo "  - Restart:       docker-compose restart"
    echo "  - Status:        docker-compose ps"
    echo ""
}

# Main
main() {
    case "${1:-start}" in
        start)
            check_prereqs
            setup_directories
            start_stack
            wait_for_services
            show_access_info
            ;;
        stop)
            log "Stopping monitoring stack..."
            docker-compose down
            log_success "Stack stopped"
            ;;
        restart)
            log "Restarting monitoring stack..."
            docker-compose restart
            log_success "Stack restarted"
            ;;
        status)
            docker-compose ps
            ;;
        logs)
            docker-compose logs -f
            ;;
        *)
            echo "Usage: $0 {start|stop|restart|status|logs}"
            exit 1
            ;;
    esac
}

main "$@"
