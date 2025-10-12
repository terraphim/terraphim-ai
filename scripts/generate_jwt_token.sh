#!/bin/bash
# Generate JWT token for TruthForge AI client authentication
# Usage: ./generate_jwt_token.sh [username] [roles]

USERNAME="${1:-ai-client}"
ROLES="${2:-authp/user}"
JWT_SHARED_KEY="${JWT_SHARED_KEY:-terraphim-jwt-shared-key-5c28cc33679085bfd8189be4cbbaf913b5b83d389f41d9d76661e2d707e60abd}"

# JWT expires in 24 hours
EXP=$(($(date +%s) + 86400))
IAT=$(date +%s)

# Create JWT header
HEADER='{"alg":"HS256","typ":"JWT"}'
HEADER_B64=$(echo -n "$HEADER" | base64 | tr -d '=' | tr '/+' '_-' | tr -d '\n')

# Create JWT payload
PAYLOAD="{\"sub\":\"$USERNAME\",\"roles\":[\"$ROLES\"],\"iat\":$IAT,\"exp\":$EXP}"
PAYLOAD_B64=$(echo -n "$PAYLOAD" | base64 | tr -d '=' | tr '/+' '_-' | tr -d '\n')

# Create signature
SIGNATURE=$(echo -n "${HEADER_B64}.${PAYLOAD_B64}" | openssl dgst -sha256 -hmac "$JWT_SHARED_KEY" -binary | base64 | tr -d '=' | tr '/+' '_-' | tr -d '\n')

# Combine to create JWT
JWT="${HEADER_B64}.${PAYLOAD_B64}.${SIGNATURE}"

echo "Generated JWT token for user: $USERNAME"
echo "Roles: $ROLES"
echo "Expires: $(date -d @$EXP)"
echo ""
echo "JWT Token:"
echo "$JWT"
echo ""
echo "Usage example:"
echo "curl -H 'Authorization: Bearer $JWT' https://alpha.truthforge.terraphim.cloud/api/v1/truthforge/analyses"
