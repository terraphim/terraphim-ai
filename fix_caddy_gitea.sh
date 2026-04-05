#!/bin/bash
# Add Gitea config to Caddyfile and reload

CADDYFILE="/home/alex/caddy_terraphim/conf/Caddyfile_auth"

# Backup current config
cp "$CADDYFILE" "$CADDYFILE.bak.$(date +%Y%m%d-%H%M%S)"

# Check if git.terraphim.cloud is already configured
if ! grep -q "git.terraphim.cloud" "$CADDYFILE"; then
    cat >> "$CADDYFILE" << 'EOF'

# Gitea Git Server
git.terraphim.cloud {
	import tls_config
	reverse_proxy localhost:3000 {
		header_up X-Real-IP {remote_host}
		header_up X-Forwarded-For {remote_host}
		header_up X-Forwarded-Proto {scheme}
	}
	log {
		output file /home/alex/caddy_terraphim/log/git.log {
			roll_size 10MiB
			roll_keep 10
			roll_keep_for 336h
		}
	}
}
EOF
    echo "Added git.terraphim.cloud configuration"

    # Reload Caddy
    cd /home/alex/caddy_terraphim && ./caddy reload --config conf/Caddyfile_auth
    echo "Caddy reloaded successfully"
else
    echo "git.terraphim.cloud already configured"
fi
