#!/bin/bash

echo "Sync remote S3 bucket to local cache"
weed -v 4 shell << EOF
remote.cache -dir=/buckets/${AWS_BUCKET}
EOF