use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use mqttbytes::v5::{Packet, SubAck, Subscribe, UnsubAck, Unsubscribe};
use tokio::sync::Notify;

pub(crate) struct SubscribeHandler {
    next_sub_id: u16,
    next_unsub_id: u16,
    pending_suback: HashMap<u16, Arc<Notify>>,
    pending_unsuback: HashMap<u16, Arc<Notify>>,
}

impl SubscribeHandler {
    pub(crate) fn new() -> SubscribeHandler {
        Self {
            next_sub_id: 0,
            next_unsub_id: 0,
            pending_suback: HashMap::new(),
            pending_unsuback: HashMap::new(),
        }
    }

    fn next_sub_id(&mut self) -> u16 {
        Self::next_id(&mut self.next_sub_id)
    }

    fn next_unsub_id(&mut self) -> u16 {
        Self::next_id(&mut self.next_unsub_id)
    }

    fn next_id(id: &mut u16) -> u16 {
        *id = id.wrapping_add(1);
        if *id == 0 {
            *id += 1;
        }
        *id
    }

    pub fn subscribe(
        &mut self,
        subscribe: &mut Subscribe,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let id = self.next_sub_id();
        subscribe.pkid = id;
        let notify = Arc::new(Notify::new());
        self.pending_suback.insert(id, notify.clone());

        Box::pin(async move {
            notify.notified().await;
        })
    }

    pub fn suback(&mut self, suback: SubAck) -> Vec<Packet> {
        let id = suback.pkid;
        // TODO check reason
        let notify = self.pending_suback.remove(&id).unwrap();
        notify.notify_one();
        Vec::new()
    }

    pub fn unsubscribe(
        &mut self,
        unsubscribe: &mut Unsubscribe,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let id = self.next_unsub_id();
        unsubscribe.pkid = id;
        let notify = Arc::new(Notify::new());
        self.pending_unsuback.insert(id, notify.clone());

        Box::pin(async move {
            notify.notified().await;
        })
    }

    pub fn unsuback(&mut self, unsuback: UnsubAck) -> Vec<Packet> {
        let id = unsuback.pkid;
        // TODO check reason
        self.pending_unsuback.remove(&id);
        Vec::new()
    }
}
