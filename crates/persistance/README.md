The goal of persistance crate is to crate a layer of abstraction for persistance layer using OpenDAL. 
the idea is that writes will go everywhere in async mode, but all reads will be from fastest operator.
see ./examples/simple_struct.rs 

save_to_one will save to single profile 
save to all will save to all available profile
load will load from fastest profile


Config file here is to define available services for persistance layer.
It is taken from: 
https://github.com/apache/incubator-opendal/tree/main/bin/oli with minor modifications:
prefix for env variables shall be TERRAPHIM_PROFILE instead of OLI_PROFILE
profile name can't have spaces

it will parse all profiles and then measure load speed for each operator, current profile config: 
```
[profiles.s3]
type = "s3"
bucket = "test"
region = "us-east-1"
endpoint = "http://rpi4node3:8333/"
access_key_id = ""
secret_access_key = ""

[profiles.sled]
type = "sled"
datadir= "/tmp/opendal/sled"

[profiles.dash]
type = "dashmap"
root = "/tmp/dashmaptest"

[profiles.rock]
type = "rocksdb"
datadir = "/tmp/opendal/rocksdb"
```

