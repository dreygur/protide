//! Sealing of CRDT entries on the gossip sync topic.
//!
//! Gossipsub announces topic subscriptions to every connected peer, and
//! any Protide instance on the LAN connects through mDNS, so the topic
//! name is no secret. What keeps a group's data inside the group is the
//! seal of `seal.rs`, bound to the topic as associated data so a seal
//! never opens on any other topic.
#![cfg(feature = "p2p-sync")]

use super::seal::{open_entry, seal_entry};
use super::*;

impl SyncEngine {
    /// `entry` sealed for our group's sync topic, or `None` for a node
    /// without a group, which has no key and so publishes nothing.
    pub(super) fn seal_for_sync(&self, entry: &CrdtEntry) -> Option<Vec<u8>> {
        let secret = self.config.group_secret.as_ref()?;
        seal_entry(secret, &self.sync_topic(), entry)
    }

    /// Publish `entry` to the group, sealed. An entry is never sent in
    /// the clear.
    pub(super) fn publish_entry(&self, entry: &CrdtEntry) {
        let Some(ref p2p) = self.p2p_sync else {
            return;
        };
        if let Some(data) = self.seal_for_sync(entry) {
            p2p.publish_sync(data);
        }
    }

    /// Open a message from the sync topic under the current group key and
    /// merge it. Anything that does not open came from outside the group,
    /// from a group we left, or was altered, and is dropped unread.
    pub(super) fn accept_sync_msg(&mut self, data: &[u8], events: &mut Vec<SyncEvent>) {
        let topic = self.sync_topic();
        let entry = (self.config.group_secret.as_ref()).and_then(|s| open_entry(s, &topic, data));
        match entry {
            Some(entry) => self.merge_remote_entry(entry, events),
            None => debug!("[P2P] Dropped a sync message that did not open"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::seal::NONCE_LEN;
    use super::*;

    const FIELD_VALUE: &str = "https://internal.example/token-4f2a";

    fn member(secret: [u8; 32]) -> SyncEngine {
        SyncEngine::new(SyncConfig {
            group_secret: Some(GroupSecret(secret)),
            ..Default::default()
        })
    }

    /// `from` makes a local change and returns it with its wire payload.
    fn publish(from: &mut SyncEngine) -> (CrdtEntry, Vec<u8>) {
        let data = format!(r#"{{"url":"{FIELD_VALUE}"}}"#);
        let entry = from.apply_local_change(DataType::Request, data);
        let wire = from.seal_for_sync(&entry).unwrap();
        (entry, wire)
    }

    /// Whether `to` merged `entry` after receiving `wire`.
    fn accepts(to: &mut SyncEngine, entry: &CrdtEntry, wire: &[u8]) -> bool {
        to.accept_sync_msg(wire, &mut Vec::new());
        to.store.get(&entry.id).is_some()
    }

    #[test]
    fn a_peer_in_the_same_group_reads_the_others_entry() {
        let (mut alice, mut bob) = (member([1; 32]), member([1; 32]));
        let (entry, wire) = publish(&mut alice);
        let mut events = Vec::new();
        bob.accept_sync_msg(&wire, &mut events);
        assert_eq!(bob.store.get(&entry.id).unwrap().data, entry.data);
        assert!(matches!(&events[..], [SyncEvent::EntryReceived(e)] if e.id == entry.id));
    }

    /// REGRESSION: entries went out as plaintext JSON on a topic every LAN
    /// peer could learn and subscribe to.
    #[test]
    fn a_peer_knowing_only_the_topic_cannot_read() {
        let mut alice = member([1; 32]);
        let (entry, wire) = publish(&mut alice);
        let plain = FIELD_VALUE.as_bytes();
        assert!(!wire.windows(plain.len()).any(|w| w == plain));
        assert!(serde_json::from_slice::<CrdtEntry>(&wire).is_err());

        let topic = alice.sync_topic();
        assert!(open_entry(&GroupSecret([2; 32]), &topic, &wire).is_none());
        assert!(!accepts(&mut member([2; 32]), &entry, &wire));
        let mut no_group = SyncEngine::new(SyncConfig::default());
        assert!(!accepts(&mut no_group, &entry, &wire));
    }

    #[test]
    fn an_entry_injected_from_outside_the_group_is_rejected() {
        let (mut alice, mut mallory) = (member([1; 32]), member([2; 32]));
        let topic = alice.sync_topic();
        let (entry, _) = publish(&mut mallory);
        let forged = seal_entry(&GroupSecret([2; 32]), &topic, &entry).unwrap();
        assert!(!accepts(&mut alice, &entry, &forged));
        let plaintext = serde_json::to_vec(&entry).unwrap();
        assert!(!accepts(&mut alice, &entry, &plaintext));
    }

    #[test]
    fn a_node_without_a_group_publishes_nothing() {
        let mut loner = SyncEngine::new(SyncConfig::default());
        let entry = loner.apply_local_change(DataType::Request, "{}".into());
        assert!(loner.seal_for_sync(&entry).is_none());
    }

    #[test]
    fn tampered_ciphertext_is_rejected() {
        let (mut alice, mut bob) = (member([1; 32]), member([1; 32]));
        let (entry, wire) = publish(&mut alice);
        for i in [0, NONCE_LEN, wire.len() - 1] {
            let mut tampered = wire.clone();
            tampered[i] ^= 1;
            assert!(!accepts(&mut bob, &entry, &tampered), "byte {i} altered");
        }
        assert!(!accepts(&mut bob, &entry, &wire[..NONCE_LEN - 1]));
        let secret = GroupSecret([1; 32]);
        assert!(open_entry(&secret, "another-topic", &wire).is_none());
        assert!(accepts(&mut bob, &entry, &wire), "the intact seal opens");
    }
}
