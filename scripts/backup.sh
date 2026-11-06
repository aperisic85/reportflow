#!/usr/bin/env bash
set -euo pipefail
mkdir -p backups
filename="backups/reportflow-$(date +%Y%m%d-%H%M%S).sql.gz"
docker compose exec -T db pg_dump -U reportflow -d reportflow | gzip > "$filename"
echo "Backup saved: $filename"
