use bytes::Bytes;
use mqttbytes::v5::{Connect, ConnectProperties, LastWill, Login};
use tokio::net::{TcpStream, ToSocketAddrs};

use crate::{Client, HandlerRouter};

pub struct ClientBuilder<Address: ToSocketAddrs> {
    address: Address,
    client_id: Option<String>,
    keep_alive: Option<u16>,
    clean_session: Option<bool>,
    last_will: Option<LastWill>,
    login: Option<Login>,

    authentication_method_and_data: Option<(String, Bytes)>,
    user_properties: Vec<(String, String)>,
}

impl<Address> ClientBuilder<Address>
where
    Address: ToSocketAddrs,
{
    pub fn new(address: Address) -> Self {
        Self {
            address,
            client_id: None,
            keep_alive: None,
            clean_session: None,
            last_will: None,
            login: None,
            authentication_method_and_data: None,
            user_properties: Vec::new(),
        }
    }

    pub fn set_client_id(&mut self, client_id: impl Into<String>) -> &mut Self {
        self.client_id = Some(client_id.into());
        self
    }

    pub fn set_keep_alive(&mut self, keep_alive: impl Into<u16>) -> &mut Self {
        self.keep_alive = Some(keep_alive.into());
        self
    }

    pub fn set_clean_session(&mut self, clean_session: bool) -> &mut Self {
        self.clean_session = Some(clean_session);
        self
    }

    pub fn set_last_will(&mut self, last_will: LastWill) -> &mut Self {
        self.last_will = Some(last_will);
        self
    }

    pub fn set_login(&mut self, login: Login) -> &mut Self {
        self.login = Some(login);
        self
    }

    pub fn set_authentication_method_and_data(&mut self, method: String, data: Bytes) -> &mut Self {
        self.authentication_method_and_data = Some((method, data));
        self
    }

    pub fn set_user_properties(
        &mut self,
        properties: impl IntoIterator<Item = (String, String)>,
    ) -> &mut Self {
        self.user_properties = properties.into_iter().collect();
        self
    }

    pub async fn build(self, publish_router: HandlerRouter) -> Client {
        let stream = TcpStream::connect(self.address).await.unwrap();

        let client_id = self.client_id.unwrap_or_else(|| "qute".to_owned());
        let mut connect = Connect::new(client_id);

        if let Some(keep_alive) = self.keep_alive {
            connect.keep_alive = keep_alive;
        }
        if let Some(clean_session) = self.clean_session {
            connect.clean_session = clean_session;
        }
        connect.last_will = self.last_will;
        connect.login = self.login;

        let mut properties = ConnectProperties {
            session_expiry_interval: None,  // defaults to 0
            receive_maximum: None,          // defaults to 65,535
            max_packet_size: None,          // defaults to no limit
            topic_alias_max: None,          // defaults to 0, i.e. no aliases allowed
            request_response_info: Some(1), // Allow response information from the server in CONNACK
            request_problem_info: Some(1),  // Allow request problem information on all packets
            user_properties: self.user_properties,
            authentication_method: None,
            authentication_data: None,
        };

        if let Some((method, data)) = self.authentication_method_and_data {
            properties.authentication_method = Some(method);
            properties.authentication_data = Some(data);
        }

        connect.properties = Some(properties);

        Client::connect(stream, publish_router, connect).await
    }
}
