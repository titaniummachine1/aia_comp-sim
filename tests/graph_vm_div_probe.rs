//! Div-by-zero semantics: the game (author-confirmed) returns IEEE
//! +inf / -inf / nan — the VM must not guard.

use aia_comp_sim::graph::load::{index_graph, RawConnection, RawGraph, RawNode, RawPort};
use aia_comp_sim::graph_vm::runtime_brain::RuntimeBrain;
use aia_comp_sim::api::DenseTeamApi;
use aia_comp_sim::brain::TeamId;
use aia_comp_sim::mode::GameSpec;

fn raw(id: &str, modifier: &str, ports: &[(&str, i32)]) -> RawNode {
    RawNode {
        id: id.to_string(),
        sid: format!("n_{id}_{}", UUID.fetch_add(1, std::sync::atomic::Ordering::SeqCst)),
        modifier: serde_json::Value::String(modifier.to_string()),
        owner_function_sid: String::new(),
        ports: ports
            .iter()
            .map(|(pid, pol)| RawPort {
                id: pid.to_string(),
                sid: format!(
                    "p_{}_{}_{}",
                    id, pid,
                    UUID.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                ),
                node_sid: String::new(),
                polarity: *pol,
            })
            .collect(),
    }
}

static UUID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[test]
fn div_by_zero_is_ieee_not_guarded() {
    let mut nodes = vec![
        raw("Float", "1", &[("Float1", 1)]),
        raw("Float", "0", &[("Float1", 1)]),
        raw("DivideFloats", "", &[("Float1", 0), ("Float2", 0), ("Float1", 1)]),
        raw("TimePlot", "", &[("String1", 0), ("Float1", 0)]),
        raw("String", "T_div", &[("String1", 1)]),
    ];
    // wire Float(1) -> Divide.Float1, Float(0) -> Divide.Float2, Divide -> TimePlot.Float1
    let div = &nodes[2];
    let div_sid = div.sid.clone();
    let div_f1 = div.ports[0].sid.clone();
    let div_f2 = div.ports[1].sid.clone();
    let div_out = div.ports[2].sid.clone();
    let tp = &nodes[3];
    let tp_f1 = tp.ports[1].sid.clone();
    nodes[0].ports[0].node_sid = nodes[0].sid.clone();
    nodes[1].ports[0].node_sid = nodes[1].sid.clone();
    nodes[4].ports[0].node_sid = nodes[4].sid.clone();
    let c = |a: String, b: String| RawConnection { port0: a, port1: b };
    let conns = vec![
        c(nodes[0].ports[0].sid.clone(), div_f1),
        c(nodes[1].ports[0].sid.clone(), div_f2),
        c(div_out, tp_f1),
        c(nodes[4].ports[0].sid.clone(), nodes[3].ports[0].sid.clone()),
    ];
    let graph = index_graph(
        RawGraph { nodes, connections: conns },
        "test".to_string(),
    );
    let mut brain = RuntimeBrain::compile_for(graph, GameSpec::tennis_builder());
    let api = DenseTeamApi::empty(TeamId::Home);
    use aia_comp_sim::TeamBrain as _;
    let _ = brain.think(&api);
    let frame = aia_comp_sim::debug_draw::snapshot();
    let div: Vec<f32> = frame.plots.iter().filter(|(n, _)| n == "T_div").map(|(_, v)| *v).collect();
    assert!(
        div.iter().any(|v| v.is_infinite() && *v > 0.0),
        "1/0 must plot +inf (game IEEE semantics), got {div:?}"
    );
}
