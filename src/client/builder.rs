use mqttbytes::v5::Connect;
use tokio::net::{TcpStream, ToSocketAddrs};

use crate::{Client, HandlerRouter};

pub struct ClientBuilder<Address: ToSocketAddrs> {
    address: Address,
    client_id: String,
}

impl<Address> ClientBuilder<Address>
where
    Address: ToSocketAddrs,
{
    pub fn new(address: Address) -> Self {
        Self {
            address,
            client_id: "qute".to_owned(),
        }
    }

    pub async fn build(self, publish_router: HandlerRouter) -> Client {
        let stream = TcpStream::connect(self.address).await.unwrap();

        let connect = Connect::new(self.client_id);

        Client::connect(stream, publish_router, connect).await
    }
}
