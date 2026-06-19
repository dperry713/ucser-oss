use etcd_client::{Client, Error};

pub struct KvStore {
    client: Client,
}

impl KvStore {
    /// Connects to the etcd cluster.
    pub async fn connect(endpoints: &[impl AsRef<str>]) -> Result<Self, Error> {
        let client = Client::connect(endpoints, None).await?;
        Ok(Self { client })
    }

    /// Puts a value into the KV store.
    pub async fn put(&mut self, key: impl Into<Vec<u8>>, value: impl Into<Vec<u8>>) -> Result<(), Error> {
        self.client.put(key, value, None).await?;
        Ok(())
    }

    /// Retrieves a value from the KV store.
    pub async fn get(&mut self, key: impl Into<Vec<u8>>) -> Result<Option<Vec<u8>>, Error> {
        let response = self.client.get(key, None).await?;
        if let Some(kv) = response.kvs().first() {
            Ok(Some(kv.value().to_vec()))
        } else {
            Ok(None)
        }
    }

    /// Deletes a key from the KV store.
    pub async fn delete(&mut self, key: impl Into<Vec<u8>>) -> Result<(), Error> {
        self.client.delete(key, None).await?;
        Ok(())
    }
}
