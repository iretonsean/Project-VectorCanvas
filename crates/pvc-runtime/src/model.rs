//! Flattened node arena built once from a loaded `pvc_ir::Module`. Hidden (`show`-false) nodes stay
//! in the arena; visibility is evaluated per update.

use pvc_ir::ast::{Module, Node};
use std::collections::HashMap;

pub struct Flat {
    /// Nodes in document order; `children` of every entry is emptied (use `kids`).
    pub nodes: Vec<Node>,
    pub kids: Vec<Vec<usize>>,
    pub parent: Vec<Option<usize>>,
    pub by_id: HashMap<String, usize>,
}

impl Flat {
    pub fn build(m: &Module) -> Result<Flat, String> {
        let mut f = Flat { nodes: vec![], kids: vec![], parent: vec![], by_id: HashMap::new() };
        fn go(f: &mut Flat, n: &Node, parent: Option<usize>) -> Result<usize, String> {
            let idx = f.nodes.len();
            let mut copy = n.clone();
            copy.children.clear();
            if f.by_id.insert(n.id.clone(), idx).is_some() {
                return Err(format!("duplicate node id {}", n.id));
            }
            f.nodes.push(copy);
            f.kids.push(vec![]);
            f.parent.push(parent);
            for c in &n.children {
                let ci = go(f, c, Some(idx))?;
                f.kids[idx].push(ci);
            }
            Ok(idx)
        }
        go(&mut f, &m.root, None)?;
        Ok(f)
    }
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}

/// Stable accesskit node id from the IR node id: FNV-1a 64 over the id bytes (deterministic on
/// every platform; collisions are rejected at load).
pub fn node_id_hash(id: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in id.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}
