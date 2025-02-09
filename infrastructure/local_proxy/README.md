# Start local SeaweedFS

```bash
weed server -filer=true -s3
```
give it a few seconds to start until it will stop throwing errors

# Configure the local SeaweedFS as proxy to S3 bucket
In another terminal, start the local configuration:
```bash
op run --no-masking --env-file="./prod.env" -- ./setup_local_proxy.sh
```
than sync the data from S3 to local SeaweedFS and back to S3

```bash
op run --no-masking --env-file="./prod.env" -- ./sync_data.sh
```

# Check that everything is working
```bash
op run --no-masking --env-file="./prod.env" -- python check_access.py
```
Polars supports scan from s3 
```
