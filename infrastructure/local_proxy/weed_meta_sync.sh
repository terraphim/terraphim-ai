#!/bin/bash

echo "Sync local changes to remote S3 bucket"
weed -v 4 shell << EOF
remote.meta.sync -dir=/buckets/${AWS_BUCKET} 
EOF