//! Boxes-and-arrows diagrams shared by the engine's `diagram` content step and the
//! quiz extension's `diagram_exercise`. Nodes sit on a small grid; the web app draws them.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagram {
    pub nodes: Vec<DiagramNode>,
    #[serde(default)]
    pub edges: Vec<DiagramEdge>,
    /// Markdown shown under the diagram.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagramNode {
    pub id: String,
    /// Required for explanation diagrams; omitted for the slots of an exercise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Markdown shown when the learner selects the node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Grid position. Defaults: row 0, column = position in the list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub col: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<NodeKind>,
    /// Exercise only: the learner places a chip here.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub slot: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Person,
    App,
    Model,
    Data,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagramEdge {
    pub from: String,
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub dashed: bool,
}

/// Which nodes may be slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotRule {
    /// Explanation diagram: every node needs a label, no slots.
    Forbidden,
    /// Exercise: at least one slot; slots carry no label.
    Required,
}

impl Diagram {
    pub fn check(&self, slots: SlotRule) -> Vec<String> {
        let mut issues = Vec::new();
        if self.nodes.is_empty() {
            issues.push("config.nodes must not be empty".to_owned());
        }
        let mut ids = BTreeSet::new();
        let mut positions = BTreeSet::new();
        for (index, node) in self.nodes.iter().enumerate() {
            if node.id.trim().is_empty() || !ids.insert(node.id.as_str()) {
                issues.push(format!(
                    "node ids must be unique and non-empty (`{}`)",
                    node.id
                ));
            }
            if !positions.insert(node.position(index)) {
                issues.push(format!("node `{}` overlaps another node", node.id));
            }
            match (slots, node.slot, &node.label) {
                (SlotRule::Forbidden, true, _) => {
                    issues.push(format!("node `{}`: slots are only for exercises", node.id));
                }
                (_, false, None) => issues.push(format!("node `{}` needs a label", node.id)),
                (SlotRule::Required, true, Some(_)) => issues.push(format!(
                    "slot `{}` must not have a label; the learner fills it in",
                    node.id
                )),
                _ => {}
            }
        }
        if slots == SlotRule::Required && !self.nodes.iter().any(|node| node.slot) {
            issues.push("an exercise diagram needs at least one node with `slot: true`".into());
        }
        for edge in &self.edges {
            for end in [&edge.from, &edge.to] {
                if !ids.contains(end.as_str()) {
                    issues.push(format!("edge refers to unknown node `{end}`"));
                }
            }
        }
        issues
    }

    pub fn slot_ids(&self) -> impl Iterator<Item = &str> {
        self.nodes
            .iter()
            .filter(|node| node.slot)
            .map(|node| node.id.as_str())
    }
}

impl DiagramNode {
    /// `(row, col)` with defaults applied; `index` is the node's position in the list.
    pub fn position(&self, index: usize) -> (u32, u32) {
        (
            self.row.unwrap_or(0),
            self.col.unwrap_or(u32::try_from(index).unwrap_or(u32::MAX)),
        )
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn diagram(value: serde_json::Value) -> Diagram {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn explanation_diagrams_need_labels_and_known_edges() {
        let issues = diagram(json!({
            "nodes": [ { "id": "a", "label": "Agent" }, { "id": "b" } ],
            "edges": [ { "from": "a", "to": "c" } ]
        }))
        .check(SlotRule::Forbidden);
        assert!(issues.iter().any(|i| i.contains("`b` needs a label")));
        assert!(issues.iter().any(|i| i.contains("unknown node `c`")));
    }

    #[test]
    fn exercises_need_unlabelled_slots() {
        let ok = diagram(json!({
            "nodes": [ { "id": "a", "label": "Agent" }, { "id": "b", "slot": true } ],
            "edges": [ { "from": "a", "to": "b" } ]
        }));
        assert!(ok.check(SlotRule::Required).is_empty());
        assert!(!ok.check(SlotRule::Forbidden).is_empty());
    }

    #[test]
    fn overlapping_positions_are_rejected() {
        let issues = diagram(json!({
            "nodes": [ { "id": "a", "label": "A", "col": 1 }, { "id": "b", "label": "B" } ]
        }))
        .check(SlotRule::Forbidden);
        assert!(issues.iter().any(|i| i.contains("overlaps")));
    }
}
