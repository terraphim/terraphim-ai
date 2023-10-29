
use opendal::{Result};
use serde::{Deserialize, Serialize};

use persistance::{Persistable, init_operator_via_map};
use persistance::config::Config;
use async_trait::async_trait;
use dirs::config_dir;
use anyhow::anyhow;

#[derive(Debug, Serialize, Deserialize)]
struct MyStruct {
    name: String,
    age: u8,
}
#[async_trait]
impl Persistable for MyStruct {
    fn new() -> Self {
        MyStruct {
            name: String::new(),
            age: 0,
        }
    }
    
    async fn save(&self) -> Result<()> {
        let op = init_operator_via_map().unwrap();
        let _= self.save_to_operator(&op).await?;
        Ok(())
    }
    
    async fn load(&mut self, key:&str) -> Result<Self> {
        let op = init_operator_via_map().unwrap();
        
        let obj = Self::load_from_operator(key, &op).await?;
        Ok(obj)
    }
    
    fn get_key(&self) -> String {
        self.name.to_ascii_lowercase()
    }
}



#[tokio::main]
async fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    let d = config_dir().ok_or_else(|| anyhow!("unknown config dir")).unwrap();
    let default_config_path = d.join("terraphim/config.toml");
    println!("Config path {:?}", default_config_path.to_str().unwrap());
    let cfg = Config::load(default_config_path.as_path()).unwrap();
    println!("Config loaded cfg: {:#?}", cfg);
    let profile = cfg.profiles["s3"].clone();
    let (op, path) = cfg.parse_location("s3:///foo/1.txt").unwrap();
    
    println!("Op {:?}", op);
    println!("Location {:?}", path);

    let obj = MyStruct {
        name: "Alice123".to_string(),
        age: 118,
    };
    
    obj.save().await?;
    println!("saved obj: {:?}", obj);
    let mut obj1 = MyStruct::new();
    let key = obj.get_key();
    println!("key: {}", key);
    obj1=obj1.load(&key).await?;
    println!("loaded obj: {:?}", obj1);
    
    // println!("{:?}", loaded_obj);
    
    Ok(())
}