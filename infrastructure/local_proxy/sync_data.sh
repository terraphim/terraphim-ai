#!/bin/bash

echo "Syncing remote metadata from S3 bucket"
weed -v 4 shell << EOL
remote.meta.sync -dir=/buckets/${AWS_BUCKET} 
EOL

echo "Caching remote S3 bucket"
weed -v 4 shell << EOF
remote.cache -dir=/buckets/${AWS_BUCKET}
EOF

echo "Syncing local SeaweedFS with  S3 bucket"
weed -v 4 filer.remote.sync -dir=/buckets/${AWS_BUCKET} &