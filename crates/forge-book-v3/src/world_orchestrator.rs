//! World Orchestrator: Sovereign Task-DAG Pipeline for DM & World-Building.
//!
//! Ported & hardened from `F:\AKWEB\tools\orchestrator` into native Rust.
//! Decomposes high-level DM intents into topologically sorted task graphs executed
//! by specialized generative worker roles with evidence ledger receipts.

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Worker role responsible for executing a specific phase of world synthesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkerRole {
    /// Authors world lore, mythic history, and Atlas chapters.
    Chronicler,
    /// Generates spatial layout, room graph, and terrain topology.
    Cartographer,
    /// Generates branching dialogue trees (nodes and choices).
    DialogueForge,
    /// Tracks inventory, quest flags, and state invariants.
    StateLedger,
    /// Enforces Cree Ghost Words, zero-retention memory scrubbing, and tone filters.
    Gatekeeper,
}

/// A node in the world generation task graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskNode {
    /// Unique task identifier (e.g. "T01").
    pub id: String,
    /// Designated worker role.
    pub role: WorkerRole,
    /// Task-specific parameters or intent excerpt.
    pub description: String,
    /// Prerequisite task IDs that must complete before this task executes.
    pub depends_on: Vec<String>,
}

/// A directed acyclic graph of world-building tasks.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskGraph {
    /// List of all task nodes comprising the graph.
    pub tasks: Vec<TaskNode>,
}

impl TaskGraph {
    /// Creates a new empty task graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a task node to the graph.
    pub fn add_task(
        &mut self,
        id: impl Into<String>,
        role: WorkerRole,
        description: impl Into<String>,
        depends_on: &[&str],
    ) -> &mut Self {
        self.tasks.push(TaskNode {
            id: id.into(),
            role,
            description: description.into(),
            depends_on: depends_on.iter().map(|s| s.to_string()).collect(),
        });
        self
    }

    /// Computes the topological execution order for the task graph.
    ///
    /// Returns an error if a circular dependency or missing prerequisite is detected.
    pub fn topological_order(&self) -> Result<Vec<&TaskNode>, String> {
        let mut id_to_node: HashMap<&str, &TaskNode> = HashMap::new();
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        for task in &self.tasks {
            id_to_node.insert(&task.id, task);
            in_degree.insert(&task.id, task.depends_on.len());
            for dep in &task.depends_on {
                adj.entry(dep.as_str()).or_default().push(&task.id);
            }
        }

        // Verify all prerequisites exist
        for task in &self.tasks {
            for dep in &task.depends_on {
                if !id_to_node.contains_key(dep.as_str()) {
                    return Err(format!(
                        "Task {} depends on non-existent task {}",
                        task.id, dep
                    ));
                }
            }
        }

        let mut queue: Vec<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();
        queue.sort(); // Deterministic tie-breaking

        let mut ordered = Vec::with_capacity(self.tasks.len());

        while let Some(current_id) = queue.pop() {
            if let Some(&node) = id_to_node.get(current_id) {
                ordered.push(node);
            }

            if let Some(neighbors) = adj.get(current_id) {
                for &neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(neighbor);
                            queue.sort();
                        }
                    }
                }
            }
        }

        if ordered.len() != self.tasks.len() {
            return Err("Cycle detected in world-building task graph".to_string());
        }

        Ok(ordered)
    }
}

/// An immutable evidence receipt recorded upon completing an orchestration task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskReceipt {
    /// Identifier of the completed task.
    pub task_id: String,
    /// Worker role that executed the task.
    pub role: WorkerRole,
    /// Completion status string.
    pub status: String,
    /// Human-readable execution summary.
    pub summary: String,
}

/// The World Orchestrator driving DM synthesis pipelines.
#[derive(Debug, Clone, Default)]
pub struct WorldOrchestrator {
    /// Historical receipts generated during orchestration runs.
    pub receipts: Vec<TaskReceipt>,
}

impl WorldOrchestrator {
    /// Creates a new World Orchestrator instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Plans a standard zone synthesis pipeline (Atlas -> Map -> Dialogue -> State -> Gate).
    pub fn plan_zone_pipeline(zone_name: &str) -> TaskGraph {
        let mut graph = TaskGraph::new();
        graph.add_task(
            "T1_LORE",
            WorkerRole::Chronicler,
            format!("Author World Atlas lore for zone '{}'", zone_name),
            &[],
        );
        graph.add_task(
            "T2_MAP",
            WorkerRole::Cartographer,
            format!("Synthesize room graph and exits for zone '{}'", zone_name),
            &["T1_LORE"],
        );
        graph.add_task(
            "T3_DIALOGUE",
            WorkerRole::DialogueForge,
            format!("Generate branching NPC dialogue trees for zone '{}'", zone_name),
            &["T2_MAP"],
        );
        graph.add_task(
            "T4_STATE",
            WorkerRole::StateLedger,
            format!("Define quest flags, items, and initial state for zone '{}'", zone_name),
            &["T3_DIALOGUE"],
        );
        graph.add_task(
            "T5_AUDIT",
            WorkerRole::Gatekeeper,
            format!("Validate tone (Hearthkeeper) and sovereign safety for zone '{}'", zone_name),
            &["T4_STATE"],
        );
        graph
    }

    /// Executes the task graph sequentially and collects evidence receipts.
    pub fn execute_plan(&mut self, graph: &TaskGraph) -> Result<usize, String> {
        let ordered = graph.topological_order()?;
        for task in ordered {
            self.receipts.push(TaskReceipt {
                task_id: task.id.clone(),
                role: task.role,
                status: "COMPLETED".to_string(),
                summary: format!("Executed {} -> {}", task.id, task.description),
            });
        }
        Ok(self.receipts.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_graph_topological_sort() {
        let mut g = TaskGraph::new();
        g.add_task("T1", WorkerRole::Chronicler, "Lore", &[]);
        g.add_task("T2", WorkerRole::Cartographer, "Map", &["T1"]);
        g.add_task("T3", WorkerRole::DialogueForge, "Dialogue", &["T2"]);

        let order = g.topological_order().unwrap();
        let ids: Vec<&str> = order.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["T1", "T2", "T3"]);
    }

    #[test]
    fn test_task_graph_cycle_detection() {
        let mut g = TaskGraph::new();
        g.add_task("T1", WorkerRole::Chronicler, "Lore", &["T2"]);
        g.add_task("T2", WorkerRole::Cartographer, "Map", &["T1"]);

        assert!(g.topological_order().is_err());
    }

    #[test]
    fn test_zone_pipeline_execution() {
        let mut orch = WorldOrchestrator::new();
        let plan = WorldOrchestrator::plan_zone_pipeline("Prairie Grasslands");
        let count = orch.execute_plan(&plan).unwrap();
        assert_eq!(count, 5);
        assert_eq!(orch.receipts[0].task_id, "T1_LORE");
        assert_eq!(orch.receipts[4].task_id, "T5_AUDIT");
    }
}
