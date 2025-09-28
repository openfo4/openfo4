use std::fmt::Write;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use petgraph::prelude::StableGraph;
use petgraph::visit::{EdgeRef, IntoEdgeReferences, IntoNodeReferences};

use crate::driver::AnimationDriver;
use crate::{BoneWeights, GraphNode};

static INIT: OnceLock<State> = OnceLock::new();

struct State {
    timer: Mutex<Instant>,
}

pub fn update(driver: &AnimationDriver) {
    let state = INIT.get_or_init(|| State {
        timer: Mutex::new(Instant::now()),
    });

    let graph = driver.graph().graph();
    let active = driver.active_nodes();

    let weights = active
        .iter()
        .map(|node| driver.global_weight(*node).unwrap())
        .collect::<Vec<_>>();

    let buf = draw(&graph, &active, &weights);

    if state.timer.lock().unwrap().elapsed() < Duration::from_secs_f32(1.0) {
        return;
    }

    *state.timer.lock().unwrap() = Instant::now();

    std::fs::write("/tmp/graph", buf.as_bytes()).unwrap();
}

pub fn draw(
    graph: &StableGraph<GraphNode, String>,
    active: &[usize],
    weights: &[&BoneWeights],
) -> String {
    let mut buf = String::from("digraph {");

    for (index, node) in graph.node_references() {
        let mut label = node.name.replace("\\", "\\\\");
        let mut color = "white";

        if let Some(index) = active.iter().position(|v| *v == node.id) {
            color = "green";

            let max_weight = match &weights[index] {
                BoneWeights::Shared(v) => *v,
                BoneWeights::Unique(v) => v.iter().copied().max_by(f32::total_cmp).unwrap_or(0.0),
            };

            if max_weight == 0.0 {
                color = "red";
            }

            write!(label, " [MW={}]", max_weight).unwrap();
        }

        writeln!(
            buf,
            "{} [label = \"{}\", fillcolor = {}]",
            index.index(),
            label.replace("[", "\\[").replace("]", "\\]"),
            color,
        )
        .unwrap();
    }

    for edge in graph.edge_references() {
        writeln!(
            buf,
            "{} -> {}",
            edge.source().index(),
            edge.target().index()
        )
        .unwrap();
    }

    writeln!(buf, "}}").unwrap();

    buf
}
