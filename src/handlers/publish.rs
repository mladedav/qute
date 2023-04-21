use std::{
    collections::{HashMap, HashSet},
    future::{self, Future},
    pin::Pin,
    sync::Arc,
};

use mqttbytes::{
    v5::{Packet, PubAck, PubComp, PubRec, PubRel, Publish},
    QoS,
};
use tokio::sync::Notify;

use crate::HandlerRouter;

pub(crate) struct SentPublishHandler {
    next_id: u16,
    pending_ack: HashMap<u16, (Publish, Arc<Notify>)>,
    pending_rec: HashMap<u16, (Publish, Arc<Notify>)>,
    pending_comp: HashSet<u16>,
}

pub(crate) struct ReceivedPublishHandler {
    pending_rel: HashMap<u16, Publish>,
    publish_router: HandlerRouter,
}

impl SentPublishHandler {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            pending_ack: HashMap::new(),
            pending_rec: HashMap::new(),
            pending_comp: HashSet::new(),
        }
    }

    pub fn publish(&mut self, publish: &mut Publish) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        match &publish.qos {
            QoS::AtMostOnce => Box::pin(future::ready(())),
            QoS::AtLeastOnce => {
                let id = self.next_id();
                publish.pkid = id;
                let notify = Arc::new(Notify::new());
                self.pending_ack
                    .insert(id, (publish.clone(), notify.clone()));
                Box::pin(async move {
                    notify.notified().await;
                })
            }
            QoS::ExactlyOnce => {
                let id = self.next_id();
                publish.pkid = id;
                let notify = Arc::new(Notify::new());
                self.pending_rec
                    .insert(id, (publish.clone(), notify.clone()));
                Box::pin(async move {
                    notify.notified().await;
                })
            }
        }
    }

    fn next_id(&mut self) -> u16 {
        self.next_id = self.next_id.wrapping_add(1);
        if self.next_id == 0 {
            self.next_id += 1;
        }
        self.next_id
    }

    pub fn puback(&mut self, puback: PubAck) -> Vec<Packet> {
        let id = puback.pkid;
        // TODO check reason
        let (_, notify) = self.pending_ack.remove(&id).unwrap();
        notify.notify_one();

        Vec::new()
    }

    pub fn pubrec(&mut self, pubrec: PubRec) -> Vec<Packet> {
        let id = pubrec.pkid;
        let (_, notify) = self.pending_rec.remove(&id).unwrap();
        notify.notify_one();
        self.pending_comp.insert(id);

        vec![Packet::PubRel(PubRel::new(id))]
    }

    pub fn pubrel(&self, _pubrel: &mut PubRel) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(future::ready(()))
    }

    pub fn pubcomp(&mut self, pubcomp: PubComp) -> Vec<Packet> {
        let id = pubcomp.pkid;
        self.pending_comp.remove(&id);

        Vec::new()
    }
}

impl ReceivedPublishHandler {
    pub fn new(publish_router: HandlerRouter) -> Self {
        Self {
            pending_rel: HashMap::new(),
            publish_router,
        }
    }

    pub async fn publish(&mut self, publish: Publish) -> Vec<Packet> {
        tracing::info!(?publish, "Received publish packet.");
        let mut reply = Vec::new();

        self.publish_router.handle(publish.clone()).await;

        match publish.qos {
            QoS::AtMostOnce => (),
            QoS::AtLeastOnce => {
                let puback = PubAck::new(publish.pkid);
                reply.push(Packet::PubAck(puback));
            }
            QoS::ExactlyOnce => {
                let pubrec = PubRec::new(publish.pkid);
                self.pending_rel.insert(publish.pkid, publish);
                reply.push(Packet::PubRec(pubrec));
            }
        }

        reply
    }

    pub(crate) fn puback(&self, _puback: &mut PubAck) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(future::ready(()))
    }

    pub(crate) fn pubrec(&self, _puback: &mut PubRec) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(future::ready(()))
    }

    pub fn pubrel(&mut self, pubrel: PubRel) -> Vec<Packet> {
        let id = pubrel.pkid;
        self.pending_rel.remove(&id);

        vec![Packet::PubComp(PubComp::new(id))]
    }

    pub fn pubcomp(&mut self, _pubcomp: &mut PubComp) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(future::ready(()))
    }
}
