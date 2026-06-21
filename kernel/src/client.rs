use tonic::transport::Channel;
use crate::execution::execution_adapter_client::ExecutionAdapterClient;
use crate::execution::{CommandRequest, CommandResponse};

#[derive(Clone)]
pub struct AdapterClient {
    client: ExecutionAdapterClient<Channel>,
}

impl AdapterClient {
    pub async fn connect(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let client = ExecutionAdapterClient::connect(url.to_string()).await?;
        Ok(Self { client })
    }

    pub async fn dispatch(
        &self,
        execution_id: String,
        command: String,
        args: Vec<String>,
        env_vars: std::collections::HashMap<String, String>,
    ) -> Result<CommandResponse, Box<dyn std::error::Error>> {
        let mut client_clone = self.client.clone();
        let request = tonic::Request::new(CommandRequest {
            execution_id,
            command,
            args,
            env_vars,
        });

        let response = client_clone.dispatch_command(request).await?;
        Ok(response.into_inner())
    }
}
