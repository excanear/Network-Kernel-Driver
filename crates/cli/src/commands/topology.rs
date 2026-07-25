use serde::Deserialize;

#[derive(Deserialize)]
struct Node {
    label: String,
    node_type: String,
}

#[derive(Deserialize)]
struct Edge {
    from: String,
    to: String,
}

#[derive(Deserialize)]
struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

pub async fn run(base_url: &str) -> anyhow::Result<()> {
    let graph: Graph = reqwest::get(format!("{base_url}/topology"))
        .await?
        .error_for_status()?
        .json()
        .await?;

    println!("Network Observatory — topology ({} nodes, {} edges)", graph.nodes.len(), graph.edges.len());
    println!();
    println!("{:<10} {}", "TYPE", "NODE");
    for node in &graph.nodes {
        println!("{:<10} {}", node.node_type, node.label);
    }
    println!();
    println!("Edges:");
    for edge in &graph.edges {
        println!("  {} -> {}", edge.from, edge.to);
    }
    Ok(())
}
