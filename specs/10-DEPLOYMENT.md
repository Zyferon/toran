# DEPLOYMENT & INFRASTRUCTURE
## How Toran Runs in Production

### Deployment Mode 1: Single Binary (Default)
The simplest deployment. One binary, one config file, one SQLite database.

```
./toran-core --config toran.toml
```

The binary listens on a Unix socket. The Python SDK connects to that socket. The dashboard (if used) connects to the REST API on a TCP port. Everything runs on one machine.

Use this for:
- Local development
- Single-server production
- Small teams (under 10 agents)

### Deployment Mode 2: Docker Compose
A `docker-compose.yml` that spins up:
- Toran core (Rust binary in distroless container)
- Toran dashboard (Next.js in Node.js container)
- Redis (for state management, if configured)
- PostgreSQL (for audit logs, if configured)
- Nginx (reverse proxy, SSL termination)

```
docker-compose up -d
```

Use this for:
- Small teams with dedicated infrastructure
- Teams that want separation of concerns
- Easier backup and monitoring

### Deployment Mode 3: Kubernetes
A Helm chart that deploys:
- Toran core as a Deployment (3 replicas for high availability)
- Toran dashboard as a Deployment (2 replicas)
- Redis as a StatefulSet (1 master, 2 replicas)
- PostgreSQL as a StatefulSet (managed by CloudNativePG operator)
- Ingress (NGINX or Traefik) with TLS
- PersistentVolumeClaims for SQLite/PostgreSQL data

Use this for:
- Enterprise deployments
- High availability requirements
- Multi-region deployments

### Deployment Mode 4: Embedded (No Separate Process)
For users who want the absolute lowest latency and simplest setup, the Rust core can be compiled as a shared library and loaded directly into the Python process via PyO3. No socket. No separate binary. The Python SDK calls Rust functions directly in the same memory space.

Use this for:
- Single-machine deployments where latency is critical
- Embedded systems
- Testing and CI/CD

Tradeoff: No hot-reloading of policies (requires Python process restart). No multi-language SDK support. But zero communication overhead.

### Reverse Proxy & SSL
Always run Toran behind a reverse proxy in production:
- Nginx: battle-tested, simple configuration
- Traefik: cloud-native, automatic Let's Encrypt
- Caddy: automatic HTTPS, simplest config

The reverse proxy handles:
- SSL/TLS termination
- Rate limiting (layer 7)
- Static asset serving (dashboard JS/CSS)
- Load balancing (if multiple core instances)

### Database Backups
- SQLite: Copy the .db file to S3/R2 daily. Use `sqlite3 .backup` for consistency.
- Redis: Use Redis persistence (RDB snapshots + AOF log). Back up the dump file.
- PostgreSQL: Use `pg_dump` daily. Store in object storage. Test restores monthly.

### Monitoring
- Prometheus: Scrape `/metrics` endpoint for request counts, latency, error rates, active approvals.
- Grafana: Visualize Prometheus metrics. Set alerts for:
  - Approval queue depth > 100 (humans are not keeping up)
  - Core latency p99 > 5ms (policy evaluation is slow)
  - Error rate > 1% (something is broken)
- UptimeRobot: Ping `/health` every 5 minutes. Alert if down.
- Sentry: Capture Rust panics and Python exceptions.

### Log Management
- Rust core: Structured JSON logs to stdout. Use `tracing` crate with `tracing-subscriber`.
- Python SDK: Structured JSON logs via Python's `logging` module.
- Dashboard: Access logs via reverse proxy.
- Aggregate: Use Vector, Fluentd, or Filebeat to ship logs to Elasticsearch, Loki, or CloudWatch.

### Scaling Strategy
- Vertical scaling: Bigger CPU for the Rust core (policy evaluation is CPU-bound). More RAM for Redis (state storage is memory-bound).
- Horizontal scaling: Run multiple Rust core instances behind a load balancer. Use Redis for shared state. Use PostgreSQL for shared audit logs.
- Shard by agent ID: If you have 1,000 agents, route agent 1-500 to core instance A, 501-1000 to core instance B. This eliminates shared state contention.
