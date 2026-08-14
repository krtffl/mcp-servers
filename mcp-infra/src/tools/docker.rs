//! `list_docker_containers` tool — Docker container listing via bollard.

use bollard::Docker;
use bollard::container::ListContainersOptions;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ContainerInfo {
    pub name: String,
    pub image: String,
    pub status: String,
    pub state: String,
    pub ports: Vec<String>,
}

/// List Docker containers, optionally filtered by name.
///
/// # Errors
///
/// Returns an error if the Docker daemon is unreachable, if the Docker API
/// call fails, or if the response fails to serialize to JSON.
pub async fn execute(name_filter: Option<&str>, show_all: bool) -> Result<String, String> {
    let docker = Docker::connect_with_defaults()
        .map_err(|e| format!("Docker connection failed: {e}. Ensure Docker is running."))?;

    let mut filters = std::collections::HashMap::new();
    if let Some(name) = name_filter {
        filters.insert("name", vec![name]);
    }

    let options = ListContainersOptions {
        all: show_all,
        filters,
        ..Default::default()
    };

    let containers = docker
        .list_containers(Some(options))
        .await
        .map_err(|e| format!("Docker API error: {e}"))?;

    let infos: Vec<ContainerInfo> = containers
        .iter()
        .map(|c| {
            let name = c.names.as_ref().and_then(|n| n.first()).map_or_else(
                || "unknown".to_string(),
                |n| n.trim_start_matches('/').to_string(),
            );

            let ports = c
                .ports
                .as_ref()
                .map(|ps| {
                    ps.iter()
                        .filter_map(|p| {
                            let private = p.private_port;
                            p.public_port
                                .map(|pub_port| format!("{pub_port}->{private}"))
                        })
                        .collect()
                })
                .unwrap_or_default();

            ContainerInfo {
                name,
                image: c.image.clone().unwrap_or_default(),
                status: c.status.clone().unwrap_or_default(),
                state: c.state.clone().unwrap_or_default(),
                ports,
            }
        })
        .collect();

    serde_json::to_string_pretty(&infos).map_err(|e| format!("JSON error: {e}"))
}
