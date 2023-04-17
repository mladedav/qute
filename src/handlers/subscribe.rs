use std::collections::HashSet;

use mqttbytes::v5::{Packet, SubAck, Subscribe, UnsubAck, Unsubscribe};

pub(crate) struct SubscribeHandler {
    next_sub_id: u16,
    next_unsub_id: u16,
    pending_suback: HashSet<u16>,
    pending_unsuback: HashSet<u16>,
}

impl SubscribeHandler {
    pub(crate) fn new() -> SubscribeHandler {
        Self {
            next_sub_id: 0,
            next_unsub_id: 0,
            pending_suback: HashSet::new(),
            pending_unsuback: HashSet::new(),
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

    pub fn subscribe(&mut self, subscribe: &mut Subscribe) {
        let id = self.next_sub_id();
        subscribe.pkid = id;
        self.pending_suback.insert(id);
    }

    pub fn suback(&mut self, suback: SubAck) -> Vec<Packet> {
        let id = suback.pkid;
        // TODO check reason
        self.pending_suback.remove(&id);
        Vec::new()
    }

    pub fn unsubscribe(&mut self, unsubscribe: &mut Unsubscribe) {
        let id = self.next_unsub_id();
        unsubscribe.pkid = id;
        self.pending_unsuback.insert(id);
    }

    pub fn unsuback(&mut self, unsuback: UnsubAck) -> Vec<Packet> {
        let id = unsuback.pkid;
        // TODO check reason
        self.pending_unsuback.remove(&id);
        Vec::new()
    }
}
