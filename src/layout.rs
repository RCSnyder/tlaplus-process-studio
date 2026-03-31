use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use petgraph::algo::toposort;
use petgraph::stable_graph::{NodeIndex, StableDiGraph};
use petgraph::Direction::Outgoing;

use crate::model::ParsedMachine;

#[derive(Clone, Debug)]
pub struct DiagramLayout {
    pub start: String,
    pub depths: BTreeMap<String, usize>,
    pub positions: BTreeMap<String, (f64, f64)>,
    pub node_rects: BTreeMap<String, (f64, f64, f64, f64)>,
    pub edge_groups: BTreeMap<(String, String), Vec<String>>,
    pub total_w: f64,
    pub total_h: f64,
    pub node_w: f64,
    pub node_h: f64,
    pub side_label_width: f64,
    pub pad: f64,
    pub init_space: f64,
    pub v_gap: f64,
}

pub fn compute_state_diagram_layout(parsed: &ParsedMachine) -> DiagramLayout {
    let start = parsed.start_state();
    let edge_groups = collect_edge_groups(parsed);
    let edge_pairs = edge_groups.keys().cloned().collect::<Vec<_>>();
    let (graph, state_to_node) = build_state_graph(parsed, &edge_pairs);
    let base_depths = assign_ranks_dfs(&graph, &state_to_node, &start);
    let ordered_base_levels = order_levels(&parsed.states, &edge_pairs, &base_depths);
    let depths = rebalance_levels_for_vertical_bias(
        &ordered_base_levels,
        &edge_pairs,
        preferred_level_width(parsed.states.len()),
    );
    let ordered_levels = order_levels(&parsed.states, &edge_pairs, &depths);

    let char_w = 7.2_f64;
    let max_label_len = parsed.states.iter().map(|s| s.len()).max().unwrap_or(8);
    let max_edge_label_len = parsed.actions.iter().map(|action| action.name.len()).max().unwrap_or(12) as f64;
    let node_w = (max_label_len as f64 * char_w + 28.0).max(100.0).min(180.0).round();
    let node_h = 34.0;
    let h_gap = 18.0;
    let v_gap = 72.0;
    let pad = 36.0;
    let init_space = 32.0;
    let side_label_width = (max_edge_label_len * 6.2).max(56.0) + 18.0;
    let left_label_gutter = (side_label_width * 0.55).clamp(46.0, 120.0);
    let right_label_gutter = (side_label_width * 0.75).clamp(58.0, 160.0);
    let level_gaps = compute_level_gaps(&ordered_levels, &depths, &edge_groups, h_gap, max_edge_label_len);

    let layout_w = ordered_levels
        .iter()
        .map(|(level, states)| {
            let gaps = level_gaps.get(level).cloned().unwrap_or_default();
            states.len() as f64 * node_w + gaps.iter().sum::<f64>()
        })
        .fold(node_w, f64::max)
        + pad * 2.0;
    let has_self_loops = edge_groups.keys().any(|(from, to)| from == to);
    let self_loop_gutter = if has_self_loops { 92.0 } else { 0.0 };
    let total_w = left_label_gutter + layout_w + right_label_gutter + self_loop_gutter;
    let total_h = (ordered_levels.len() as f64) * (node_h + v_gap) - v_gap + pad * 2.0 + init_space;

    let mut positions = BTreeMap::new();
    for (level, states) in &ordered_levels {
        let gaps = level_gaps.get(level).cloned().unwrap_or_default();
        let row_w = states.len() as f64 * node_w + gaps.iter().sum::<f64>();
        let x0 = left_label_gutter + (layout_w - row_w) / 2.0 + node_w / 2.0;
        let mut x = x0;
        for (index, state) in states.iter().enumerate() {
            let y = pad + init_space + (*level as f64) * (node_h + v_gap) + node_h / 2.0;
            positions.insert(state.clone(), (x, y));
            if let Some(gap) = gaps.get(index) {
                x += node_w + *gap;
            }
        }
    }

    let node_rects = positions
        .iter()
        .map(|(state, &(x, y))| (state.clone(), (x, y, node_w / 2.0 + 4.0, node_h / 2.0 + 4.0)))
        .collect();

    DiagramLayout {
        start,
        depths,
        positions,
        node_rects,
        edge_groups,
        total_w,
        total_h,
        node_w,
        node_h,
        side_label_width,
        pad,
        init_space,
        v_gap,
    }
}

fn preferred_level_width(state_count: usize) -> usize {
    if state_count <= 1 {
        1
    } else {
        (((state_count as f64).sqrt() * 0.9).ceil() as usize).clamp(2, 4)
    }
}

fn collect_edge_groups(parsed: &ParsedMachine) -> BTreeMap<(String, String), Vec<String>> {
    let mut edge_groups: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for action in &parsed.actions {
        for from in &action.from {
            for to in &action.to {
                edge_groups
                    .entry((from.clone(), to.clone()))
                    .or_default()
                    .push(action.name.clone());
            }
        }
    }
    edge_groups
}

fn build_state_graph(
    parsed: &ParsedMachine,
    edge_pairs: &[(String, String)],
) -> (StableDiGraph<String, ()>, HashMap<String, NodeIndex>) {
    let mut graph = StableDiGraph::<String, ()>::new();
    let mut state_to_node = HashMap::new();

    for state in &parsed.states {
        let node = graph.add_node(state.clone());
        state_to_node.insert(state.clone(), node);
    }

    let mut seen_edges = HashSet::new();
    for (from, to) in edge_pairs {
        if from == to {
            continue;
        }
        if seen_edges.insert((from.clone(), to.clone())) {
            if let (Some(&src), Some(&dst)) = (state_to_node.get(from), state_to_node.get(to)) {
                graph.add_edge(src, dst, ());
            }
        }
    }

    (graph, state_to_node)
}

/// Classify edges as forward (tree/cross) or back-edges via DFS from the start state.
/// Back-edges are those that point to an ancestor in the DFS tree (i.e. cycle-closing edges).
fn classify_edges_dfs(
    graph: &StableDiGraph<String, ()>,
    state_to_node: &HashMap<String, NodeIndex>,
    start: &str,
) -> (HashSet<(NodeIndex, NodeIndex)>, HashSet<(NodeIndex, NodeIndex)>) {
    let mut forward_edges = HashSet::new();
    let mut back_edges = HashSet::new();

    // DFS states: 0 = white (unvisited), 1 = gray (on stack), 2 = black (finished)
    let mut color: HashMap<NodeIndex, u8> = HashMap::new();
    for &node in state_to_node.values() {
        color.insert(node, 0);
    }

    // Iterative DFS using an explicit stack
    let start_node = match state_to_node.get(start) {
        Some(&n) => n,
        None => return (forward_edges, back_edges),
    };

    // We may have unreachable nodes, so run DFS from start first, then from any remaining whites
    let mut roots = vec![start_node];
    // Add all other nodes as potential secondary roots (for disconnected components)
    let mut other_nodes: Vec<NodeIndex> = state_to_node.values().copied().filter(|&n| n != start_node).collect();
    other_nodes.sort_by_key(|n| graph[*n].clone());
    roots.extend(other_nodes);

    for &root in &roots {
        if color[&root] != 0 {
            continue;
        }

        // Stack entries: (node, neighbor_iterator_index, is_entry)
        // We use a manual stack to avoid recursion limits on large graphs
        let mut stack: Vec<(NodeIndex, Vec<NodeIndex>, usize)> = Vec::new();
        color.insert(root, 1); // gray
        let neighbors: Vec<NodeIndex> = graph.neighbors_directed(root, Outgoing).collect();
        stack.push((root, neighbors, 0));

        while let Some((node, ref neighbors_list, ref mut idx)) = stack.last_mut() {
            let node = *node;
            if *idx < neighbors_list.len() {
                let neighbor = neighbors_list[*idx];
                *idx += 1;
                match color[&neighbor] {
                    0 => {
                        // Tree edge (forward) — descend
                        forward_edges.insert((node, neighbor));
                        color.insert(neighbor, 1); // gray
                        let next_neighbors: Vec<NodeIndex> = graph.neighbors_directed(neighbor, Outgoing).collect();
                        stack.push((neighbor, next_neighbors, 0));
                    }
                    1 => {
                        // Back edge — neighbor is an ancestor on the current DFS path
                        back_edges.insert((node, neighbor));
                    }
                    _ => {
                        // Cross or forward edge to already-finished node — treat as forward
                        forward_edges.insert((node, neighbor));
                    }
                }
            } else {
                // Done with this node
                color.insert(node, 2); // black
                stack.pop();
            }
        }
    }

    (forward_edges, back_edges)
}

/// Assign ranks using DFS back-edge reversal + longest-path layering.
/// This is the standard Sugiyama phase 1-2 pipeline.
fn assign_ranks_dfs(
    graph: &StableDiGraph<String, ()>,
    state_to_node: &HashMap<String, NodeIndex>,
    start: &str,
) -> BTreeMap<String, usize> {
    let (forward_edges, back_edges) = classify_edges_dfs(graph, state_to_node, start);

    // Build a DAG: keep forward edges as-is, reverse back-edges
    let mut dag = StableDiGraph::<String, ()>::new();
    let mut dag_node_map: HashMap<NodeIndex, NodeIndex> = HashMap::new();
    for &orig in state_to_node.values() {
        let dag_n = dag.add_node(graph[orig].clone());
        dag_node_map.insert(orig, dag_n);
    }
    for &(src, dst) in &forward_edges {
        dag.add_edge(dag_node_map[&src], dag_node_map[&dst], ());
    }
    for &(src, dst) in &back_edges {
        // Reverse the back-edge so it goes dst → src in the DAG
        dag.add_edge(dag_node_map[&dst], dag_node_map[&src], ());
    }

    // Longest-path ranking on the DAG
    let topo = toposort(&dag, None).unwrap_or_else(|_| {
        // If toposort fails (residual cycle), fall back to BFS ranking from start
        bfs_ranking(graph, state_to_node, start)
            .into_iter()
            .collect::<Vec<_>>()
            .into_iter()
            .flat_map(|(_, _)| None::<NodeIndex>) // won't be used
            .collect()
    });

    // If toposort succeeded, do longest-path
    if !topo.is_empty() || dag.node_count() == 0 {
        let mut rank: HashMap<NodeIndex, usize> = HashMap::new();
        let dag_start = state_to_node.get(start).and_then(|n| dag_node_map.get(n)).copied();

        // Initialize all ranks to 0
        for &dag_n in dag_node_map.values() {
            rank.insert(dag_n, 0);
        }

        // Process in topological order: rank[v] = max(rank[v], rank[u] + 1) for each edge u→v
        for &dag_n in &topo {
            let cur_rank = rank[&dag_n];
            for neighbor in dag.neighbors_directed(dag_n, Outgoing) {
                let new_rank = cur_rank + 1;
                let entry = rank.entry(neighbor).or_insert(0);
                if new_rank > *entry {
                    *entry = new_rank;
                }
            }
        }

        // Ensure start is at rank 0 by shifting: subtract start's rank from all
        let start_rank = dag_start.and_then(|n| rank.get(&n)).copied().unwrap_or(0);
        let min_rank = rank.values().copied().min().unwrap_or(0);
        let shift = start_rank.min(min_rank); // Usually start_rank if it's the earliest

        // Build the node_to_orig reverse mapping
        let orig_to_dag: HashMap<NodeIndex, NodeIndex> = dag_node_map.iter().map(|(&k, &v)| (k, v)).collect();

        let mut result = BTreeMap::new();
        for (state, &orig_node) in state_to_node {
            if let Some(&dag_n) = orig_to_dag.get(&orig_node) {
                let r = rank.get(&dag_n).copied().unwrap_or(0);
                result.insert(state.clone(), r.saturating_sub(shift));
            } else {
                result.insert(state.clone(), 0);
            }
        }

        // Normalize: ensure start is 0 and compact gaps
        let start_r = result.get(start).copied().unwrap_or(0);
        for val in result.values_mut() {
            *val = val.saturating_sub(start_r);
        }

        result
    } else {
        // Fallback: BFS-based ranking
        bfs_ranking(graph, state_to_node, start)
    }
}

/// BFS-based ranking fallback: assigns rank = shortest path distance from start.
fn bfs_ranking(
    graph: &StableDiGraph<String, ()>,
    state_to_node: &HashMap<String, NodeIndex>,
    start: &str,
) -> BTreeMap<String, usize> {
    let mut ranks = BTreeMap::new();
    let start_node = match state_to_node.get(start) {
        Some(&n) => n,
        None => return ranks,
    };

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back((start_node, 0usize));
    visited.insert(start_node);

    while let Some((node, rank)) = queue.pop_front() {
        ranks.insert(graph[node].clone(), rank);
        for neighbor in graph.neighbors_directed(node, Outgoing) {
            if visited.insert(neighbor) {
                queue.push_back((neighbor, rank + 1));
            }
        }
    }

    // Handle unreachable nodes
    let max_rank = ranks.values().copied().max().unwrap_or(0);
    for (state, _) in state_to_node {
        ranks.entry(state.clone()).or_insert(max_rank + 1);
    }

    ranks
}

fn rebalance_levels_for_vertical_bias(
    ordered_levels: &BTreeMap<usize, Vec<String>>,
    edge_pairs: &[(String, String)],
    preferred_width: usize,
) -> BTreeMap<String, usize> {
    // Build a map of all current ranks for quick lookup
    let mut current_ranks = BTreeMap::new();
    for (rank, states) in ordered_levels {
        for state in states {
            current_ranks.insert(state.clone(), *rank);
        }
    }

    // Only consider forward edges (from lower rank to higher rank) as predecessors.
    // Back-edges (higher to lower rank) should NOT pull nodes down.
    let mut incoming_forward = HashMap::<String, Vec<String>>::new();
    for (from, to) in edge_pairs {
        if from != to {
            let from_rank = current_ranks.get(from).copied().unwrap_or(0);
            let to_rank = current_ranks.get(to).copied().unwrap_or(0);
            if from_rank <= to_rank {
                incoming_forward.entry(to.clone()).or_default().push(from.clone());
            }
        }
    }

    let mut adjusted_depths = BTreeMap::new();
    let mut occupancy = BTreeMap::<usize, usize>::new();

    for (base_rank, states) in ordered_levels {
        for state in states {
            let mut target_rank = *base_rank;
            if let Some(predecessors) = incoming_forward.get(state) {
                for predecessor in predecessors {
                    if let Some(&pred_rank) = adjusted_depths.get(predecessor) {
                        target_rank = target_rank.max(pred_rank + 1);
                    }
                }
            }

            while occupancy.get(&target_rank).copied().unwrap_or(0) >= preferred_width {
                target_rank += 1;
            }

            adjusted_depths.insert(state.clone(), target_rank);
            *occupancy.entry(target_rank).or_default() += 1;
        }
    }

    adjusted_depths
}

fn order_levels(
    states: &[String],
    edge_pairs: &[(String, String)],
    depths: &BTreeMap<String, usize>,
) -> BTreeMap<usize, Vec<String>> {
    let mut levels = BTreeMap::<usize, Vec<String>>::new();
    for state in states {
        if let Some(&rank) = depths.get(state) {
            levels.entry(rank).or_default().push(state.clone());
        }
    }
    for states_at_level in levels.values_mut() {
        states_at_level.sort();
    }

    let mut level_vec = levels.into_iter().collect::<Vec<_>>();
    let sweep_count = (states.len()).clamp(12, 24);
    for _ in 0..sweep_count {
        for idx in 1..level_vec.len() {
            let adjacent_rank = level_vec[idx - 1].0;
            let order_lookup = order_lookup(&level_vec);
            reorder_level(&mut level_vec[idx].1, adjacent_rank, edge_pairs, depths, &order_lookup);
        }
        for idx in (0..level_vec.len().saturating_sub(1)).rev() {
            let adjacent_rank = level_vec[idx + 1].0;
            let order_lookup = order_lookup(&level_vec);
            reorder_level(&mut level_vec[idx].1, adjacent_rank, edge_pairs, depths, &order_lookup);
        }
    }

    level_vec.into_iter().collect()
}

fn order_lookup(levels: &[(usize, Vec<String>)]) -> HashMap<String, f64> {
    levels
        .iter()
        .flat_map(|(_, states)| {
            states
                .iter()
                .enumerate()
                .map(|(idx, state)| (state.clone(), idx as f64))
        })
        .collect()
}

fn reorder_level(
    level_states: &mut Vec<String>,
    adjacent_rank: usize,
    edge_pairs: &[(String, String)],
    depths: &BTreeMap<String, usize>,
    order_lookup: &HashMap<String, f64>,
) {
    let current_order = level_states
        .iter()
        .enumerate()
        .map(|(idx, state)| (state.clone(), idx as f64))
        .collect::<HashMap<_, _>>();

    level_states.sort_by(|left, right| {
        let left_score = barycenter(left, adjacent_rank, edge_pairs, depths, &order_lookup)
            .unwrap_or_else(|| *current_order.get(left).unwrap_or(&0.0));
        let right_score = barycenter(right, adjacent_rank, edge_pairs, depths, &order_lookup)
            .unwrap_or_else(|| *current_order.get(right).unwrap_or(&0.0));
        left_score
            .partial_cmp(&right_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.cmp(right))
    });
}

fn barycenter(
    state: &str,
    adjacent_rank: usize,
    edge_pairs: &[(String, String)],
    depths: &BTreeMap<String, usize>,
    order_lookup: &HashMap<String, f64>,
) -> Option<f64> {
    let mut neighbors = Vec::new();
    for (from, to) in edge_pairs {
        if from == state {
            if depths.get(to).copied() == Some(adjacent_rank) {
                if let Some(order) = order_lookup.get(to) {
                    neighbors.push(*order);
                }
            }
        } else if to == state {
            if depths.get(from).copied() == Some(adjacent_rank) {
                if let Some(order) = order_lookup.get(from) {
                    neighbors.push(*order);
                }
            }
        }
    }

    if neighbors.is_empty() {
        None
    } else {
        Some(neighbors.iter().sum::<f64>() / neighbors.len() as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Action;

    #[test]
    fn vertical_bias_spills_wide_rank_downward() {
        let parsed = ParsedMachine {
            module_name: "Test".to_string(),
            states: vec![
                "Start".to_string(),
                "A".to_string(),
                "B".to_string(),
                "C".to_string(),
                "D".to_string(),
                "E".to_string(),
                "F".to_string(),
            ],
            init_state: Some("Start".to_string()),
            actions: vec![
                Action { name: "ToA".to_string(), from: vec!["Start".to_string()], to: vec!["A".to_string()], comment: None },
                Action { name: "ToB".to_string(), from: vec!["Start".to_string()], to: vec!["B".to_string()], comment: None },
                Action { name: "ToC".to_string(), from: vec!["Start".to_string()], to: vec!["C".to_string()], comment: None },
                Action { name: "ToD".to_string(), from: vec!["Start".to_string()], to: vec!["D".to_string()], comment: None },
                Action { name: "ToE".to_string(), from: vec!["Start".to_string()], to: vec!["E".to_string()], comment: None },
                Action { name: "ToF".to_string(), from: vec!["Start".to_string()], to: vec!["F".to_string()], comment: None },
            ],
            invariants: Vec::new(),
            comments: Vec::new(),
            warnings: Vec::new(),
        };

        let layout = compute_state_diagram_layout(&parsed);
        let mut counts = BTreeMap::<usize, usize>::new();
        for rank in layout.depths.values() {
            *counts.entry(*rank).or_default() += 1;
        }

        assert!(counts.values().copied().max().unwrap_or(0) <= preferred_level_width(parsed.states.len()));
        assert!(layout.depths.values().copied().max().unwrap_or(0) > 1);
    }
}

fn compute_level_gaps(
    ordered_levels: &BTreeMap<usize, Vec<String>>,
    depths: &BTreeMap<String, usize>,
    edge_groups: &BTreeMap<(String, String), Vec<String>>,
    base_gap: f64,
    max_edge_label_len: f64,
) -> BTreeMap<usize, Vec<f64>> {
    let mut gaps = BTreeMap::new();

    for (level, states) in ordered_levels {
        let mut row_gaps = Vec::new();
        for pair in states.windows(2) {
            let left = &pair[0];
            let right = &pair[1];
            let has_forward = edge_groups.contains_key(&(left.clone(), right.clone()));
            let has_backward = edge_groups.contains_key(&(right.clone(), left.clone()));
            let local_label_len = edge_groups
                .get(&(left.clone(), right.clone()))
                .into_iter()
                .flatten()
                .chain(edge_groups.get(&(right.clone(), left.clone())).into_iter().flatten())
                .map(|label| label.len() as f64)
                .fold(0.0, f64::max);

            let shared_rank_neighbors = [left, right]
                .into_iter()
                .map(|state| {
                    edge_groups
                        .keys()
                        .filter(|(from, to)| {
                            from != to
                                && (from == state || to == state)
                                && depths.get(from).copied() == Some(*level)
                                && depths.get(to).copied() == Some(*level)
                        })
                        .count()
                })
                .sum::<usize>();

            let mut gap = base_gap;
            if has_forward || has_backward {
                gap = gap.max(36.0);
            }
            if has_forward && has_backward {
                gap = gap.max(54.0);
            }
            if shared_rank_neighbors > 0 {
                gap = gap.max(28.0 + (shared_rank_neighbors as f64 - 1.0).max(0.0) * 8.0);
            }
            if local_label_len > 0.0 {
                gap = gap.max((local_label_len.min(max_edge_label_len) * 0.9) + 18.0);
            }

            row_gaps.push(gap.min(84.0));
        }
        gaps.insert(*level, row_gaps);
    }

    gaps
}