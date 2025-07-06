use std::fmt::Write;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use petgraph::prelude::StableGraph;
use petgraph::visit::{EdgeRef, IntoEdgeReferences, IntoNodeReferences};

use crate::GraphNode;
use crate::driver::AnimationDriver;

static INIT: OnceLock<State> = OnceLock::new();

struct State {
    timer: Mutex<Instant>,
}

pub fn update(driver: &AnimationDriver) {
    let state = INIT.get_or_init(|| State {
        timer: Mutex::new(Instant::now()),
    });

    let graph = driver.graph.graph();
    let active = driver.active_nodes();

    let buf = draw(&graph, &active);

    if state.timer.lock().unwrap().elapsed() < Duration::from_secs_f32(1.0) {
        return;
    }

    *state.timer.lock().unwrap() = Instant::now();

    std::fs::write("/tmp/graph", buf.as_bytes()).unwrap();
}

pub fn draw(graph: &StableGraph<GraphNode, String>, active: &[usize]) -> String {
    let mut buf = String::from("digraph {");

    for (index, node) in graph.node_references() {
        let color = if active.contains(&node.id) {
            "green"
        } else {
            "white"
        };

        writeln!(
            buf,
            "{} [label = \"{}\", fillcolor = {}]",
            index.index(),
            node.name.replace("\\", "\\\\"),
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
