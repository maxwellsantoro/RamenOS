//! Prerequisites Graph — generates dependency graph from queue items.

use serde::{Deserialize, Serialize};

use crate::prelude::*;

use crate::queue_item::{Prereq, QueueItemV0};

/// Node in the prerequisites graph.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraphNode {
    /// Node identifier (program_id for queue items, "prereq:{kind}:{name}" for prereqs).
    pub id: String,

    /// Node type: "queue_item" or "prereq".
    pub node_type: String,

    /// Human-readable label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Edge in the prerequisites graph.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraphEdge {
    /// Source node ID (the blocked item).
    pub from: String,

    /// Target node ID (the blocking prereq).
    pub to: String,

    /// Edge label (e.g., "blocked_by").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Prerequisites graph.
#[derive(Debug, Serialize, Deserialize)]
pub struct PrereqGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

impl PrereqGraph {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Add queue items to the graph.
    pub fn add_queue_items(&mut self, items: &[QueueItemV0]) {
        let mut prereq_ids: Vec<String> = Vec::new();

        for item in items {
            // Add queue item node
            self.nodes.push(GraphNode {
                id: item.program_id.clone(),
                node_type: "queue_item".into(),
                label: Some(format!(
                    "{} → {}",
                    item.program_id,
                    item.target_level.as_str()
                )),
            });

            // Add edges to prereqs
            for prereq in &item.prereqs {
                let prereq_id = prereq_node_id(prereq);
                if !prereq_ids.contains(&prereq_id) {
                    prereq_ids.push(prereq_id.clone());
                }

                self.edges.push(GraphEdge {
                    from: item.program_id.clone(),
                    to: prereq_id,
                    label: Some("blocked_by".into()),
                });
            }
        }

        // Add prereq nodes
        for prereq_id in prereq_ids {
            self.nodes.push(GraphNode {
                id: prereq_id.clone(),
                node_type: "prereq".into(),
                label: Some(prereq_id.replace("prereq:", "")),
            });
        }
    }

    /// Get all prereqs that block multiple queue items (high leverage).
    pub fn high_leverage_prereqs(&self) -> Vec<(String, usize)> {
        let mut result: Vec<(String, usize)> = Vec::new();

        for edge in &self.edges {
            if edge.to.starts_with("prereq:") {
                let found = result.iter_mut().find(|(id, _)| *id == edge.to);
                if let Some((_, count)) = found {
                    *count += 1;
                } else {
                    result.push((edge.to.clone(), 1));
                }
            }
        }

        result.retain(|(_, count)| *count > 1);
        result.sort_by_key(|item| core::cmp::Reverse(item.1));
        result
    }

    /// Export to DOT format for visualization.
    pub fn to_dot(&self) -> String {
        let mut lines = Vec::new();
        lines.push("digraph prereqs {".to_string());
        lines.push("  rankdir=LR;".to_string());
        lines.push("  node [shape=box];".to_string());

        // Add nodes with styling
        for node in &self.nodes {
            let label = node.label.as_deref().unwrap_or(&node.id);
            let style = if node.node_type == "prereq" {
                "style=filled,fillcolor=lightcoral"
            } else {
                "style=filled,fillcolor=lightblue"
            };
            lines.push(format!(
                "  \"{}\" [label=\"{}\",{}];",
                node.id,
                label.replace("\"", "\\\""),
                style
            ));
        }

        // Add edges
        for edge in &self.edges {
            let label_attr = match &edge.label {
                Some(l) => format!(" [label=\"{}\"]", l),
                None => String::new(),
            };
            lines.push(format!(
                "  \"{}\" -> \"{}\"{};",
                edge.from, edge.to, label_attr
            ));
        }

        lines.push("}".to_string());
        lines.join("\n")
    }
}

impl Default for PrereqGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate node ID for a prereq.
fn prereq_node_id(prereq: &Prereq) -> String {
    format!("prereq:{}:{}", prereq.kind, prereq.name)
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::queue_item::{QueueItemEvidence, ScoringInputs, TargetLevel};

    fn sample_items() -> Vec<QueueItemV0> {
        vec![
            QueueItemV0 {
                schema_version: 1,
                program_id: "app.one".into(),
                target_level: TargetLevel::Posix,
                evidence: QueueItemEvidence {
                    scenario_traces: vec!["sha256:aaa".into()],
                    observed_caps: None,
                    protocol_traces: vec![],
                },
                prereqs: vec![Prereq {
                    kind: "portal".into(),
                    name: "clipboard".into(),
                    notes: None,
                }],
                scoring: ScoringInputs {
                    vote_weight: 3,
                    leverage: 3,
                    reuse: 3,
                    effort: 3,
                    risk: 3,
                },
                priority: 3.0,
                explanation: vec![],
            },
            QueueItemV0 {
                schema_version: 1,
                program_id: "app.two".into(),
                target_level: TargetLevel::Native,
                evidence: QueueItemEvidence {
                    scenario_traces: vec!["sha256:bbb".into()],
                    observed_caps: None,
                    protocol_traces: vec![],
                },
                prereqs: vec![
                    Prereq {
                        kind: "portal".into(),
                        name: "clipboard".into(),
                        notes: None,
                    },
                    Prereq {
                        kind: "harness".into(),
                        name: "audio.capture".into(),
                        notes: None,
                    },
                ],
                scoring: ScoringInputs {
                    vote_weight: 2,
                    leverage: 2,
                    reuse: 2,
                    effort: 2,
                    risk: 2,
                },
                priority: 2.0,
                explanation: vec![],
            },
        ]
    }

    #[test]
    fn build_graph() {
        let items = sample_items();
        let mut graph = PrereqGraph::new();
        graph.add_queue_items(&items);

        // Should have 2 queue item nodes + 2 prereq nodes (clipboard shared)
        assert_eq!(graph.nodes.len(), 4);
        // Should have 3 edges (app.one→clipboard, app.two→clipboard, app.two→audio)
        assert_eq!(graph.edges.len(), 3);
    }

    #[test]
    fn high_leverage_prereqs_finds_clipboard() {
        let items = sample_items();
        let mut graph = PrereqGraph::new();
        graph.add_queue_items(&items);

        let high_leverage = graph.high_leverage_prereqs();
        assert_eq!(high_leverage.len(), 1);
        assert!(high_leverage[0].0.contains("clipboard"));
        assert_eq!(high_leverage[0].1, 2);
    }

    #[test]
    fn to_dot_produces_valid_output() {
        let items = sample_items();
        let mut graph = PrereqGraph::new();
        graph.add_queue_items(&items);

        let dot = graph.to_dot();
        assert!(dot.starts_with("digraph prereqs"));
        assert!(dot.contains("app.one"));
        assert!(dot.contains("clipboard"));
    }
}
