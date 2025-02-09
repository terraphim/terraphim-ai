import boto3
from botocore.exceptions import ClientError
import os

"""
This script checks access to an S3 bucket both directly and through a proxy. It attempts to list objects in the bucket to verify access.
"""

def check_s3_proxy(bucket_name, endpoint_url):
    try:
        s3 = boto3.client('s3', region_name=os.environ.get('AWS_REGION'), endpoint_url=endpoint_url)
        print(s3.list_objects_v2(Bucket=bucket_name, MaxKeys=1))
        return True
    except Exception as e:
        print(f"Error accessing S3 proxy: {e}")
        return False
def check_s3_access(bucket_name):
    try:
        # Create an S3 client
        s3 = boto3.client('s3', region_name=os.environ.get('AWS_REGION'))
        
        # Try to list objects in the bucket
        print(s3.list_objects_v2(Bucket=bucket_name, MaxKeys=1))
        
        print(f"Successfully accessed S3 bucket: {bucket_name}")
        # print endpoint url
        print(s3.meta.endpoint_url)
        return True
    except ClientError as e:
        print(f"Error accessing S3 bucket: {e}")
        return False

# Replace with your actual bucket name
bucket_name = os.environ.get('AWS_BUCKET')

credentials_check=check_s3_access(bucket_name)
if credentials_check:
    print("Credentials check passed")
else:
    print("Credentials check failed")

proxy_check=check_s3_proxy(bucket_name, "http://localhost:8333/")
if proxy_check:
    print("Proxy check passed")
else:
    print("Proxy check failed")

# checking writing into the bucket
import polars as pl
import s3fs

df = pl.DataFrame({
    "foo": ["a", "b", "c", "d", "d"],
    "bar": [1, 2, 3, 4, 5],
})
# testing writing into the bucket directly
print("Testing writing into the bucket directly")
fs = s3fs.S3FileSystem()
destination = f"s3://{bucket_name}/test_file.parquet"

try:
    # write parquet
    with fs.open(destination, mode='wb') as f:
        df.write_parquet(f)
    print("Writing into the bucket directly passed")
except Exception as e:
    print(f"Error writing into the bucket directly: {e}")

try:
    #  testing writing into the bucket through the proxy
    print("Testing writing into the bucket through the proxy")
    fs_proxy = s3fs.S3FileSystem(anon=False, endpoint_url="http://localhost:8333/")
    destination = f"s3://{bucket_name}/test_file_proxy2.parquet"

    # write parquet
    with fs_proxy.open(destination, mode='wb') as f:
        df.write_parquet(f)
    print("Writing into the bucket through the proxy passed")
except Exception as e:
    print(f"Error writing into the bucket through the proxy: {e}")

try:
    # reading from the bucket
    print("Reading from the bucket")
    df = pl.read_parquet(f"s3://{bucket_name}/test_file.parquet")
    print(df)
    print("Reading from the bucket passed")
except Exception as e:
    print(f"Error reading from the bucket: {e}")

try:
    # reading from the bucket through the proxy
    print("Reading from the bucket through the proxy")
    with fs_proxy.open(f"s3://{bucket_name}/test_file_proxy.parquet", mode='rb') as f:
        df = pl.read_parquet(f)
        print(df)
        print("Reading from the bucket through the proxy passed")
except Exception as e:
    print(f"Error reading from the bucket through the proxy: {e}")

# Scan parquet file from the bucket through the proxy
print("Scan parquet file from the bucket through the proxy")


storage_options = {
    "aws_access_key_id": os.environ.get('AWS_ACCESS_KEY_ID'),
    "aws_secret_access_key": os.environ.get('AWS_SECRET_ACCESS_KEY'),
    "aws_region": os.environ.get('AWS_REGION'),
}
source = f"s3://{bucket_name}/*.parquet"
df = pl.scan_parquet(source, storage_options=storage_options)
print(df)
print("Scan parquet file from the bucket through the proxy passed")