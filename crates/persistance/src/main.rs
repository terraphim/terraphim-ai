
use opendal::{Result, Operator};
use serde::{Deserialize, Serialize};

use persistance::{Persistable};
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
        let op = &self.load_config().await?.1;
        let _= self.save_to_operator(&op).await?;
        Ok(())
    }
    
    async fn load(&mut self, key:&str) -> Result<Self> {
        let op = &self.load_config().await?.1;
        
        let obj = self.load_from_operator(key, &op).await?;
        Ok(obj)
    }
    
    fn get_key(&self) -> String {
        self.name.to_ascii_lowercase()
    }
}



#[tokio::main]
async fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    // let d = config_dir().ok_or_else(|| anyhow!("unknown config dir")).unwrap();
    // let default_config_path = d.join("terraphim/config.toml");
    // println!("Config path {:?}", default_config_path.to_str().unwrap());
    // let cfg = Config::load(default_config_path.as_path()).unwrap();
    // println!("Config loaded cfg: {:#?}", cfg);
    // let profile = cfg.profiles["s3"].clone();
    // println!("Config loaded profile: {:#?}", profile);
    // let ops = cfg.parse_profiles().await?;
    // println!("Config loaded ops: {:#?}", ops);
    // let fastest_op = ops
    // .iter()
    // .min_by_key(|op| op.1)
    // .ok_or_else(|| anyhow!("No operators provided")).unwrap();
    // println!("fastest_op: {:#?}", fastest_op);

    let obj = MyStruct {
        name: "Alice123".to_string(),
        age: 118,
    };
    let (ops, fastest_op)=obj.load_config().await?;
    println!("fastest_op: {:#?}", fastest_op);
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