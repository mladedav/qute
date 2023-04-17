use std::collections::{HashMap, HashSet};

use mqttbytes::{
    v5::{Packet, PubAck, PubComp, PubRec, PubRel, Publish},
    QoS,
};

pub(crate) struct SentPublishHandler {
    next_id: u16,
    pending_ack: HashMap<u16, Publish>,
    pending_rec: HashMap<u16, Publish>,
    pending_comp: HashSet<u16>,
}

pub(crate) struct ReceivedPublishHandler {
    pending_rel: HashMap<u16, Publish>,
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

    pub fn publish(&mut self, publish: &mut Publish) {
        match publish.qos {
            QoS::AtMostOnce => (),
            QoS::AtLeastOnce => {
                let id = self.next_id();
                publish.pkid = id;
                self.pending_ack.insert(id, publish.clone());
            }
            QoS::ExactlyOnce => {
                let id = self.next_id();
                publish.pkid = id;
                self.pending_rec.insert(id, publish.clone());
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
        self.pending_ack.remove(&id);

        Vec::new()
    }

    pub fn pubrec(&mut self, pubrec: PubRec) -> Vec<Packet> {
        let id = pubrec.pkid;
        self.pending_rec.remove(&id);
        self.pending_comp.insert(id);

        vec![Packet::PubRel(PubRel::new(id))]
    }

    pub fn pubrel(&self, _packet: &mut PubRel) {}

    pub fn pubcomp(&mut self, pubcomp: PubComp) -> Vec<Packet> {
        let id = pubcomp.pkid;
        self.pending_comp.remove(&id);

        Vec::new()
    }
}

impl ReceivedPublishHandler {
    pub fn new() -> Self {
        Self {
            pending_rel: HashMap::new(),
        }
    }

    pub fn publish(&mut self, publish: Publish) -> Vec<Packet> {
        tracing::info!(?publish, "Received publish packet.");
        let mut reply = Vec::new();

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

    pub(crate) fn puback(&self, _puback: &mut PubAck) {}

    pub(crate) fn pubrec(&self, _puback: &mut PubRec) {}

    pub fn pubrel(&mut self, pubrel: PubRel) -> Vec<Packet> {
        let id = pubrel.pkid;
        self.pending_rel.remove(&id);

        vec![Packet::PubComp(PubComp::new(id))]
    }

    pub fn pubcomp(&mut self, _pubcomp: &mut PubComp) {}
}
