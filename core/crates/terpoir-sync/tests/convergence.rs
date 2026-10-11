// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Property tests: devices converge regardless of merge order.

// Test code may unwrap.
#![allow(clippy::unwrap_used)]

use proptest::prelude::*;
use std::collections::BTreeMap;
use terpoir_crypto::{MasterKey, RecordKey};
use terpoir_model::{DeviceId, Document, Hlc, RecordId, RecordKind, Stamp, Value};
use terpoir_store::SealedRecord;
use terpoir_sync::{
    Blob, Clock, MemoryReplica, apply_changes, container_key, decode_blob, encode_blob,
    merge_document,
};

const KEYS: [&str; 6] = [
    "name",
    "rating",
    "note",
    "effects/relaxed",
    "effects/sleepy",
    "lines/a",
];
const DEVICES: usize = 3;

fn value_strategy() -> impl Strategy<Value = Value> {
    prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        (-3i64..4).prop_map(Value::Int),
        prop::sample::select(vec!["a", "b", "c"]).prop_map(|s| Value::Text(s.to_owned())),
        (0i64..3).prop_map(|n| Value::Tuple(vec![Value::Int(n), Value::Null])),
    ]
}

/// A random document with fields stamped by random (possibly equal)
/// stamps, all with the same ID, for algebraic properties.
fn doc_strategy() -> impl Strategy<Value = Document> {
    prop::collection::vec(
        (0..KEYS.len(), value_strategy(), 0u64..6, 0u32..2, 0u8..3),
        0..8,
    )
    .prop_map(|fields| {
        let mut doc = Document::new(RecordId::from_bytes([7; 16]), RecordKind::Session);
        for (key, value, ms, counter, device) in fields {
            let stamp = Stamp {
                hlc: Hlc {
                    wall_ms: ms,
                    counter,
                },
                device: DeviceId::from_bytes([device; 16]),
            };
            // Keep the newest stamp per key so the generated document is
            // something a real device could hold.
            if doc.field(KEYS[key]).is_none_or(|f| f.stamp < stamp) {
                doc.set(KEYS[key], value, stamp);
            }
        }
        doc
    })
}

fn merged(a: &Document, b: &Document) -> Document {
    let mut out = a.clone();
    merge_document(&mut out, b).unwrap();
    out
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn merge_is_commutative(a in doc_strategy(), b in doc_strategy()) {
        prop_assert_eq!(merged(&a, &b), merged(&b, &a));
    }

    #[test]
    fn merge_is_associative(a in doc_strategy(), b in doc_strategy(), c in doc_strategy()) {
        prop_assert_eq!(merged(&merged(&a, &b), &c), merged(&a, &merged(&b, &c)));
    }

    #[test]
    fn merge_is_idempotent(a in doc_strategy(), b in doc_strategy()) {
        let ab = merged(&a, &b);
        prop_assert_eq!(merged(&ab, &b), ab.clone());
        prop_assert_eq!(merged(&a, &a), a);
    }
}

#[derive(Clone, Debug)]
enum Op {
    Create {
        device: usize,
        fields: Vec<(usize, Value)>,
    },
    Edit {
        device: usize,
        record: usize,
        key: usize,
        value: Value,
    },
    Delete {
        device: usize,
        record: usize,
    },
    Sync {
        from: usize,
        to: usize,
    },
    Tick {
        ms: u64,
    },
}

fn op_strategy() -> impl Strategy<Value = Op> {
    let device = 0..DEVICES;
    prop_oneof![
        2 => (device.clone(), prop::collection::vec((0..KEYS.len(), value_strategy()), 0..4))
            .prop_map(|(device, fields)| Op::Create { device, fields }),
        6 => (device.clone(), any::<usize>(), 0..KEYS.len(), value_strategy())
            .prop_map(|(device, record, key, value)| Op::Edit { device, record, key, value }),
        1 => (device.clone(), any::<usize>())
            .prop_map(|(device, record)| Op::Delete { device, record }),
        2 => (device.clone(), device).prop_map(|(from, to)| Op::Sync { from, to }),
        2 => (0u64..2_000).prop_map(|ms| Op::Tick { ms }),
    ]
}

struct Device {
    clock: Clock,
    /// Offset of this device's wall clock from true time; may be large and
    /// negative (a phone with a wrong clock).
    skew_ms: i64,
    replica: MemoryReplica,
}

impl Device {
    fn now(&self, true_ms: u64) -> u64 {
        true_ms.saturating_add_signed(self.skew_ms)
    }

    /// Applies another device's full state, as reading its blobs would.
    fn receive(&mut self, state: &MemoryReplica, true_ms: u64) {
        let now = self.now(true_ms);
        for doc in state.docs.values() {
            if let Some(stamp) = doc.latest_stamp() {
                self.clock.observe(stamp.hlc, now);
            }
        }
        apply_changes(
            &mut self.replica,
            state.docs.values().cloned(),
            state.deleted.iter().copied(),
        )
        .unwrap();
    }
}

fn simulate(ops: &[Op], skews: &[i64; DEVICES], delivery: &[Vec<usize>]) -> Vec<MemoryReplica> {
    let mut devices: Vec<Device> = (0..DEVICES)
        .map(|i| Device {
            clock: Clock::new(DeviceId::from_bytes([i as u8 + 1; 16]), Hlc::default()),
            skew_ms: skews[i],
            replica: MemoryReplica::default(),
        })
        .collect();
    let mut true_ms: u64 = 1_000_000;
    let mut next_id: u64 = 0;

    for op in ops {
        match op {
            Op::Create { device, fields } => {
                next_id += 1;
                let mut id = [0u8; 16];
                id[..8].copy_from_slice(&next_id.to_be_bytes());
                let d = &mut devices[*device];
                let stamp = d.clock.stamp(d.now(true_ms));
                let mut doc = Document::new(RecordId::from_bytes(id), RecordKind::Session);
                for (key, value) in fields {
                    doc.set(KEYS[*key], value.clone(), stamp);
                }
                d.replica.docs.insert(doc.id(), doc);
            }
            Op::Edit {
                device,
                record,
                key,
                value,
            } => {
                let d = &mut devices[*device];
                let ids: Vec<RecordId> = d.replica.docs.keys().copied().collect();
                if ids.is_empty() {
                    continue;
                }
                let id = ids[record % ids.len()];
                let stamp = d.clock.stamp(d.now(true_ms));
                d.replica
                    .docs
                    .get_mut(&id)
                    .unwrap()
                    .set(KEYS[*key], value.clone(), stamp);
            }
            Op::Delete { device, record } => {
                let d = &mut devices[*device];
                let ids: Vec<RecordId> = d.replica.docs.keys().copied().collect();
                if ids.is_empty() {
                    continue;
                }
                let id = ids[record % ids.len()];
                d.replica.docs.remove(&id);
                d.replica.deleted.insert(id);
            }
            Op::Sync { from, to } => {
                if from != to {
                    let state = devices[*from].replica.clone();
                    devices[*to].receive(&state, true_ms);
                }
            }
            Op::Tick { ms } => true_ms += ms,
        }
    }

    // Final exchange: every device receives every device's state, each in
    // its own random order.
    let snapshots: Vec<MemoryReplica> = devices.iter().map(|d| d.replica.clone()).collect();
    for (device, order) in devices.iter_mut().zip(delivery) {
        for &from in order {
            device.receive(&snapshots[from], true_ms);
        }
    }
    devices.into_iter().map(|d| d.replica).collect()
}

fn delivery_strategy() -> impl Strategy<Value = Vec<Vec<usize>>> {
    // For each device, a shuffled delivery order of all devices' snapshots,
    // with duplicates (redelivery must be harmless).
    prop::collection::vec(
        prop::collection::vec(0..DEVICES, DEVICES..DEVICES * 2).prop_flat_map(|extra| {
            let mut all: Vec<usize> = (0..DEVICES).collect();
            all.extend(extra);
            Just(all).prop_shuffle()
        }),
        DEVICES,
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn devices_converge_regardless_of_order(
        ops in prop::collection::vec(op_strategy(), 0..60),
        skews in prop::array::uniform3(-86_400_000i64..86_400_000),
        delivery in delivery_strategy(),
    ) {
        let replicas = simulate(&ops, &skews, &delivery);
        for r in &replicas[1..] {
            prop_assert_eq!(r, &replicas[0]);
        }
        // Deleted records never survive anywhere.
        for id in &replicas[0].deleted {
            prop_assert!(!replicas[0].docs.contains_key(id));
        }
    }

    #[test]
    fn delivery_order_does_not_change_the_result(
        ops in prop::collection::vec(op_strategy(), 0..60),
        skews in prop::array::uniform3(-86_400_000i64..86_400_000),
        first in delivery_strategy(),
        second in delivery_strategy(),
    ) {
        prop_assert_eq!(simulate(&ops, &skews, &first), simulate(&ops, &skews, &second));
    }
}

/// The same convergence, end to end through encrypted, padded blobs.
#[test]
fn changes_survive_the_blob_round_trip() {
    let master = MasterKey::generate().unwrap();
    let container = container_key(&master).unwrap();
    let device = DeviceId::from_bytes([9; 16]);
    let mut clock = Clock::new(device, Hlc::default());

    let mut keys = BTreeMap::new();
    let mut docs = Vec::new();
    for i in 0..5 {
        let mut doc = Document::new(RecordId::random().unwrap(), RecordKind::Brand);
        doc.set(
            "name",
            Value::Text(format!("brand {i}")),
            clock.stamp(1_000),
        );
        keys.insert(doc.id(), RecordKey::generate().unwrap());
        docs.push(doc);
    }
    let sealed: Vec<SealedRecord> = docs
        .iter()
        .map(|doc| {
            let key = &keys[&doc.id()];
            let mut aad = doc.id().as_bytes().to_vec();
            aad.push(doc.kind().code());
            SealedRecord {
                id: doc.id(),
                kind: doc.kind(),
                wrapped_key: master.wrap_record_key(key, &doc.id()).unwrap(),
                ciphertext: key.seal(&aad, &doc.encode().unwrap()).unwrap(),
            }
        })
        .collect();
    let deleted = RecordId::random().unwrap();
    let blob = Blob {
        device,
        sequence: 1,
        records: sealed,
        deletions: vec![deleted],
    };
    let bytes = encode_blob(&container, &blob).unwrap();
    let received = decode_blob(&container, &bytes).unwrap();

    let mut replica = MemoryReplica::default();
    let opened: Vec<Document> = received
        .records
        .iter()
        .map(|r| r.open(&master).unwrap())
        .collect();
    apply_changes(&mut replica, opened, received.deletions.iter().copied()).unwrap();
    assert_eq!(replica.docs.len(), 5);
    for doc in &docs {
        assert_eq!(replica.docs.get(&doc.id()), Some(doc));
    }
    assert!(replica.deleted.contains(&deleted));
}
