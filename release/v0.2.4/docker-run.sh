#!/bin/bash
# Terraphim AI Docker Runner Script v0.2.3

set -e

# Configuration
IMAGE_NAME="ghcr.io/terraphim/terraphim-server:v0.2.3"
CONTAINER_NAME="terraphim-server"
DATA_DIR="$HOME/.local/share/terraphim"
CONFIG_DIR="$HOME/.config/terraphim"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_header() {
    echo -e "${BLUE}🚀 Terraphim AI Docker Runner${NC}"
}

# Check if Docker is installed
check_docker() {
    if ! command -v docker &> /dev/null; then
        print_error "Docker is not installed. Please install Docker first."
        print_status "Installation guide: https://docs.docker.com/get-docker/"
        exit 1
    fi

    if ! docker info &> /dev/null; then
        print_error "Docker daemon is not running. Please start Docker first."
        exit 1
    fi
}

# Setup directories
setup_directories() {
    print_status "Setting up data directories..."
    mkdir -p "$DATA_DIR"
    mkdir -p "$CONFIG_DIR"

    # Create default config if it doesn't exist
    if [ ! -f "$CONFIG_DIR/config.json" ]; then
        cat > "$CONFIG_DIR/config.json" << 'EOF'
{
  "name": "Terraphim Engineer",
  "relevance_function": "TerraphimGraph",
  "theme": "spacelab",
  "haystacks": [
    {
      "name": "Local Documents",
      "service": "Ripgrep",
      "location": "/home/terraphim/data",
      "extra_parameters": {
        "glob": "*.md,*.txt,*.rst"
      }
    }
  ]
}
EOF
        print_status "Created default configuration at $CONFIG_DIR/config.json"
    fi
}

# Pull the Docker image
pull_image() {
    print_status "Pulling Terraphim Docker image..."
    docker pull "$IMAGE_NAME"
}

# Stop existing container
stop_container() {
    if docker ps -q -f name="$CONTAINER_NAME" | grep -q .; then
        print_status "Stopping existing container..."
        docker stop "$CONTAINER_NAME" || true
        docker rm "$CONTAINER_NAME" || true
    fi
}

# Run the container
run_container() {
    print_status "Starting Terraphim server container..."

    docker run -d \
        --name "$CONTAINER_NAME" \
        -p 8000:8000 \
        -v "$CONFIG_DIR:/home/terraphim/.config/terraphim" \
        -v "$DATA_DIR:/home/terraphim/data" \
        --restart unless-stopped \
        "$IMAGE_NAME"

    # Wait for the container to start
    print_status "Waiting for server to start..."
    sleep 5

    # Check if the container is running
    if docker ps -q -f name="$CONTAINER_NAME" | grep -q .; then
        print_status "✓ Terraphim server is running!"
        print_status "  Server URL: http://localhost:8000"
        print_status "  Health Check: http://localhost:8000/health"
    else
        print_error "Failed to start Terraphim server"
        docker logs "$CONTAINER_NAME" 2>&1 | tail -20
        exit 1
    fi
}

# Show status
show_status() {
    echo ""
    print_status "🎉 Terraphim AI is running!"
    echo ""
    echo "📋 Quick Commands:"
    echo "  Check status:    docker logs $CONTAINER_NAME"
    echo "  Stop server:    docker stop $CONTAINER_NAME"
    echo "  Restart server: docker restart $CONTAINER_NAME"
    echo ""
    echo "🌐 Web Interface:"
    echo "  http://localhost:8000"
    echo ""
    echo "📡 API Endpoint:"
    echo "  http://localhost:8000/api"
    echo ""
    echo "📁 Data Directory:"
    echo "  Config: $CONFIG_DIR"
    echo "  Data:   $DATA_DIR"
}

# Cleanup function
cleanup() {
    if [ "$1" = "--cleanup" ]; then
        print_status "Cleaning up..."
        stop_container
        docker rmi "$IMAGE_NAME" 2>/dev/null || true
        print_status "Cleanup completed."
        exit 0
    fi
}

# Show logs
show_logs() {
    if [ "$1" = "--logs" ]; then
        docker logs -f "$CONTAINER_NAME"
        exit 0
    fi
}

# Main function
main() {
    print_header

    # Handle special commands
    case "$1" in
        --cleanup)
            cleanup
            ;;
        --logs)
            show_logs
            ;;
        --help)
            echo "Terraphim AI Docker Runner"
            echo ""
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --help     Show this help message"
            echo "  --logs     Show container logs"
            echo "  --cleanup  Stop and remove container and image"
            echo ""
            echo "Examples:"
            echo "  $0              # Start Terraphim server"
            echo "  $0 --logs       # Show logs"
            echo "  $0 --cleanup    # Cleanup containers and images"
            exit 0
            ;;
    esac

    check_docker
    setup_directories
    pull_image
    stop_container
    run_container
    show_status
}

main "$@"
