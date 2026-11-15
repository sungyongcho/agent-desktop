use crate::refs::RefMap;
use crate::{RefEntry, RefEntryIdentity};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

const MISSING_CHAIN: &str = "RefMap names a scope chain it does not hold";

/// The stored form of a `RefMap`. Each distinct scope chain is written once in
/// `chains`, and `chain_of` maps a ref id to its chain's index. Refmaps saved
/// before this form carry their chains inline on the entries and still load.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RefMapWire {
    inner: HashMap<String, RefEntry>,
    counter: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    chains: Vec<Vec<RefEntryIdentity>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    chain_of: BTreeMap<String, usize>,
}

impl From<RefMap> for RefMapWire {
    fn from(map: RefMap) -> Self {
        let RefMap { mut inner, counter } = map;
        let mut chains: Vec<Vec<RefEntryIdentity>> = Vec::new();
        let mut index_by_key: HashMap<String, usize> = HashMap::new();
        let mut chain_of = BTreeMap::new();
        for (ref_id, entry) in inner.iter_mut() {
            if entry.scope.chain.is_empty() {
                continue;
            }
            let chain = std::mem::take(&mut entry.scope.chain);
            let key = serde_json::to_string(&chain).ok();
            let known = key.as_ref().and_then(|key| index_by_key.get(key).copied());
            let index = match known {
                Some(index) => index,
                None => {
                    chains.push(chain);
                    let index = chains.len() - 1;
                    if let Some(key) = key {
                        index_by_key.insert(key, index);
                    }
                    index
                }
            };
            chain_of.insert(ref_id.clone(), index);
        }
        Self {
            inner,
            counter,
            chains,
            chain_of,
        }
    }
}

impl TryFrom<RefMapWire> for RefMap {
    type Error = String;

    fn try_from(wire: RefMapWire) -> Result<Self, Self::Error> {
        let RefMapWire {
            mut inner,
            counter,
            chains,
            chain_of,
        } = wire;
        for (ref_id, index) in chain_of {
            let chain = chains.get(index).ok_or(MISSING_CHAIN)?;
            let entry = inner.get_mut(&ref_id).ok_or(MISSING_CHAIN)?;
            entry.scope.chain = chain.clone();
        }
        Ok(Self { inner, counter })
    }
}
