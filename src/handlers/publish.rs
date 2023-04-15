use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use mqttbytes::{
    v5::{Packet, PubAck, PubComp, PubRec, PubRel, Publish},
    QoS,
};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::connection::Connection;

pub(crate) struct PublishHandler<R, W> {
    connection: Arc<Connection<R, W>>,
    pending_ack: HashMap<u16, Publish>,
    pending_rec: HashMap<u16, Publish>,
    pending_rel: HashMap<u16, Publish>,
    pending_comp: HashSet<u16>,
}

impl<R, W> PublishHandler<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    pub fn new(connection: Arc<Connection<R, W>>) -> Self {
        Self {
            connection,
            pending_ack: HashMap::new(),
            pending_rec: HashMap::new(),
            pending_rel: HashMap::new(),
            pending_comp: HashSet::new(),
        }
    }
    pub async fn recv_publish(&mut self, publish: Publish) {
        tracing::info!(?publish, "Received publish packet.");
        match publish.qos {
            QoS::AtMostOnce => (),
            QoS::AtLeastOnce => {
                let puback = PubAck::new(publish.pkid);
                self.connection.send(Packet::PubAck(puback)).await.unwrap();
            }
            QoS::ExactlyOnce => {
                let pubrec = PubRec::new(publish.pkid);
                self.pending_rel.insert(publish.pkid, publish);
                self.connection.send(Packet::PubRec(pubrec)).await.unwrap();
            }
        }
    }

    pub fn send_publish(&mut self, publish: Publish) {
        match publish.qos {
            QoS::AtMostOnce => (),
            QoS::AtLeastOnce => {
                let id = publish.pkid;
                self.pending_ack.insert(id, publish);
            }
            QoS::ExactlyOnce => {
                let id = publish.pkid;
                self.pending_rec.insert(id, publish);
            }
        }
    }

    pub fn puback(&mut self, puback: PubAck) {
        let id = puback.pkid;
        // TODO check reason
        self.pending_ack.remove(&id);
    }

    pub async fn pubrec(&mut self, pubrec: PubRec) {
        let id = pubrec.pkid;
        self.pending_rec.remove(&id);
        self.pending_comp.insert(id);
        self.connection.send(Packet::PubRel(PubRel::new(id))).await;
    }

    pub async fn pubrel(&mut self, pubrel: PubRel) {
        let id = pubrel.pkid;
        self.pending_rel.remove(&id);
        self.connection
            .send(Packet::PubComp(PubComp::new(id)))
            .await;
    }

    pub fn pubcomp(&mut self, pubcomp: PubComp) {
        let id = pubcomp.pkid;
        self.pending_comp.remove(&id);
    }
}
