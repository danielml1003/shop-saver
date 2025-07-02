# Shop Saver - Docker Setup

This guide explains how to run the Shop Saver application using Docker.

## Architecture

The application consists of three main components:

1. **PostgreSQL Database** - Stores the processed store and item data
2. **Python Service** - Downloads store data and saves XML files
3. **Rust Backend** - Processes XML files and stores data in the database

## Prerequisites

- Docker Desktop installed and running
- Docker Compose (included with Docker Desktop)

## Quick Start

1. **Clone/Navigate to the project directory:**
   ```bash
   cd shop-saver
   ```

2. **Build and start all services:**
   ```bash
   docker-compose up --build
   ```

3. **To run in the background:**
   ```bash
   docker-compose up --build -d
   ```

## Services

### Database (PostgreSQL)
- **Container name:** `shop-saver-db`
- **Port:** `5432` (exposed to host)
- **Database:** `shop_saver`
- **Username:** `postgres`
- **Password:** `password`

### Python Service
- **Container name:** `shop-saver-python`
- **Function:** Downloads store data and saves XML files
- **Output:** XML files saved to shared volume

### Rust Backend
- **Container name:** `shop-saver-backend`
- **Function:** Monitors XML files and processes them into the database
- **Watches:** `/app/downloads` directory (shared volume)

## Management Commands

### View logs
```bash
# All services
docker-compose logs

# Specific service
docker-compose logs python-service
docker-compose logs rust-backend
docker-compose logs postgres
```

### Stop services
```bash
docker-compose down
```

### Stop and remove volumes (WARNING: This will delete all data)
```bash
docker-compose down -v
```

### Restart a specific service
```bash
docker-compose restart python-service
```

### Execute commands in containers
```bash
# Access PostgreSQL
docker-compose exec postgres psql -U postgres -d shop_saver

# Access Python service bash
docker-compose exec python-service bash

# Access Rust backend bash
docker-compose exec rust-backend bash
```

## Data Volumes

- **postgres_data:** Persistent PostgreSQL database storage
- **shared_downloads:** Shared directory for XML files between Python service and Rust backend

## Environment Variables

You can customize the setup by modifying environment variables in `docker-compose.yml`:

### Database Configuration
- `POSTGRES_DB`: Database name (default: shop_saver)
- `POSTGRES_USER`: Database username (default: postgres)
- `POSTGRES_PASSWORD`: Database password (default: password)

### Rust Backend Configuration
- `DATABASE_URL`: PostgreSQL connection string
- `WATCH_DIRECTORY`: Directory to monitor for XML files
- `RUST_LOG`: Logging level (info, debug, warn, error)

## Development

### Rebuilding after code changes

If you make changes to the Python service:
```bash
docker-compose build python-service
docker-compose up python-service
```

If you make changes to the Rust backend:
```bash
docker-compose build rust-backend
docker-compose up rust-backend
```

### Accessing the database
```bash
# Connect to PostgreSQL
docker-compose exec postgres psql -U postgres -d shop_saver

# View tables
\dt

# View stores
SELECT * FROM stores;

# View recent items
SELECT * FROM items ORDER BY processed_at DESC LIMIT 10;
```

## Troubleshooting

### Check service status
```bash
docker-compose ps
```

### View detailed logs
```bash
docker-compose logs --follow rust-backend
```

### Reset everything
```bash
docker-compose down -v
docker-compose up --build
```

### Common Issues

1. **Port 5432 already in use:** Stop any local PostgreSQL service or change the port in docker-compose.yml

2. **Permission denied errors:** Make sure Docker has access to the project directory

3. **Out of disk space:** Clean up Docker with `docker system prune`

## Production Considerations

For production deployment, consider:

1. **Security:**
   - Change default database password
   - Use environment files for secrets
   - Limit exposed ports

2. **Persistence:**
   - Backup database volumes regularly
   - Use external volumes for critical data

3. **Monitoring:**
   - Add health checks
   - Set up log aggregation
   - Monitor resource usage

4. **Scaling:**
   - Use Docker Swarm or Kubernetes for orchestration
   - Separate database to managed service
   - Add load balancers if needed
