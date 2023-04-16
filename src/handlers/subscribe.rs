use std::collections::HashSet;

use mqttbytes::v5::{Unsubscribe, UnsubAck, SubAck, Subscribe, Packet};

pub(crate) struct SubscribeHandler {
    pending_suback: HashSet<u16>,
    pending_unsuback: HashSet<u16>,
}

impl SubscribeHandler {
    pub(crate) fn new() -> SubscribeHandler {
        Self {
            pending_suback: HashSet::new(),
            pending_unsuback: HashSet::new(),
        }
    }

    pub fn subscribe(&mut self, subscribe: Subscribe) -> Vec<Packet> {
        let id = subscribe.pkid;
        self.pending_suback.insert(id);
        Vec::new()
    }

    pub fn suback(&mut self, suback: SubAck) -> Vec<Packet> {
        let id = suback.pkid;
        // TODO check reason
        self.pending_suback.remove(&id);
        Vec::new()
    }

    pub fn unsubscribe(&mut self, unsubscribe: Unsubscribe) -> Vec<Packet> {
        let id = unsubscribe.pkid;
        self.pending_unsuback.insert(id);
        Vec::new()
    }

    pub fn unsuback(&mut self, unsuback: UnsubAck) -> Vec<Packet> {
        let id = unsuback.pkid;
        // TODO check reason
        self.pending_unsuback.remove(&id);
        Vec::new()
    }
}
