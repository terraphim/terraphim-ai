# Terraphim Settings Crate
The purpose of this crate is to provide per-device settings for the Terraphim AI. It uses twelf crate to merge settings from environment and settings file.
The logic of twelf crate Layer is to merge settings from enviroment and settings file. 
User shall be able to override default settings without modify the code, by modifying either settings TERRAPHIM_SERVER_HOSTNAME in enviroment or modify value after "-" in settings file ./default/settings.toml.

Each deployment - desktop or server - has its own default settings file.
 The settings file is located in the default folder in the root of the project. The settings file is named settings.toml. 
 The settings file is a TOML file with the following structure:

```toml
server_hostname = "${TERRAPHIM_SERVER_HOSTNAME}-127.0.0.1:8000"
api_endpoint="${TERRAPHIM_SERVER_API_ENDPOINT}-http://localhost:8000/api"

[profiles.s3]
type = "s3"
bucket = "test"
region = "${TERRAPHIM_PROFILE_S3_REGION:-us-east-1}"
endpoint = "${TERRAPHIM_PROFILE_S3_ENDPOINT:-http://rpi4node3:8333/}"
access_key_id = "${AWS_ACCESS_KEY_ID}"
secret_access_key = "${AWS_SECRET_ACCESS_KEY}"

[profiles.sled]
type = "sled"
datadir= "/tmp/opendal/sled"

[profiles.dash]
type = "dashmap"
root = "/tmp/dashmaptest"

[profiles.rock]
type = "rocksdb"
datadir = "/tmp/opendal/rocksdb" 

[profiles.atomicserver]
endpoint = "${TERRAPHIM_PROFILE_ATOMICSERVER}"
type = "atomicserver"
private_key = "${TERRAPHIM_PROFILE_ATOMICSERVER_PRIVATE_KEY}"
public_key = "${TERRAPHIM_PROFILE_ATOMICSERVER_PUBLIC_KEY}"
parent_resource_id="${TERRAPHIM_PROFILE_ATOMICSERVER}"
```
one first start of the application, settings crate checks if the settings file exists. 
If it does not exist, it will copy the settings file from the default folder to ~/.config/terraphim/settings.toml (on Mac and Linux).