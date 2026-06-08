#!/usr/bin/env bash
# Apply the standard Toran filesystem permissions.
#
#   0755  binaries, directories
#   0644  source, configs, examples
#   0600  databases, secret material (when created)
#   0660  sockets (when created)
#
# Idempotent. Safe to re-run.

set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

chmod -R u+rwX,go+rX,g-w .         # default: dirs 0755, files 0644
chmod +x target/release/toran 2>/dev/null || true
chmod +x scripts/*.sh 2>/dev/null || true
chmod +x sdk/examples/*.py 2>/dev/null || true

# Make sure policy files stay world-readable (they're config).
chmod 0644 policies/*.yaml

# If a database or socket exists, tighten them.
[ -f toran.db ] && chmod 0600 toran.db
[ -f toran.db-wal ] && chmod 0600 toran.db-wal
[ -f toran.db-shm ] && chmod 0600 toran.db-shm
[ -S /tmp/toran.sock ] && chmod 0660 /tmp/toran.sock 2>/dev/null || true

echo "permissions applied"
ls -la target/release/toran policies/ 2>/dev/null | head -20
