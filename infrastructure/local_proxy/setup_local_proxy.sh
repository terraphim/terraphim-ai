#!/bin/bash
# weed server -s3 &
# Configure S3-compatible access
echo "Configuring S3-compatible access"
weed shell << EOF
s3.configure \
  -user me \
  -access_key=${AWS_ACCESS_KEY_ID} \
  -secret_key=${AWS_SECRET_ACCESS_KEY} \
  -buckets=${AWS_BUCKET} \
  -actions=Read,Write,List,Tagging,Admin \
  -apply
EOF

echo "Configuring remote S3 access"
weed shell << EOF
remote.configure \
  -name=s5 \
  -type=s3 \
  -s3.access_key=${AWS_ACCESS_KEY_ID} \
  -s3.secret_key=${AWS_SECRET_ACCESS_KEY} \
  -s3.region=${AWS_REGION} \
  -s3.storage_class=STANDARD
EOF

echo "Mounting remote S3 bucket"
weed shell << EOF
remote.mount \
  -dir=/buckets/${AWS_BUCKET} \
  -remote=s5/${AWS_BUCKET} \
  -nonempty
EOF

