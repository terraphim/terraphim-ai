use serde::{Deserialize, Serialize};

use async_trait::async_trait;
use persistence::{Persistable, Result};

#[derive(Debug, Serialize, Deserialize)]
struct MyStruct {
    name: String,
    age: u8,
}
#[async_trait]
impl Persistable for MyStruct {
    fn new(name: String) -> Self {
        MyStruct { name, age: 0 }
    }

    async fn save_to_one(&self, profile_name: &str) -> Result<()> {
        self.save_to_profile(profile_name).await?;
        Ok(())
    }

    // saves to all profiles
    async fn save(&self) -> Result<()> {
        let _op = &self.load_config().await?.1;
        let _ = self.save_to_all().await?;
        Ok(())
    }

    async fn load(&mut self, key: &str) -> Result<Self> {
        let op = &self.load_config().await?.1;

        let obj = self.load_from_operator(key, &op).await?;
        Ok(obj)
    }

    fn get_key(&self) -> String {
        self.name.clone()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    let obj = MyStruct {
        name: "No vampire".to_string(),
        age: 110,
    };
    obj.save_to_one("s3").await?;
    obj.save().await?;
    println!("saved obj: {:?} to all", obj);
    let (_ops, fastest_op) = obj.load_config().await?;
    println!("fastest_op: {:#?}", fastest_op);

    let mut obj1 = MyStruct::new("obj".to_string());
    let key = obj.get_key();
    println!("key: {}", key);
    obj1 = obj1.load(&key).await?;
    println!("loaded obj: {:?}", obj1);

    Ok(())
}
