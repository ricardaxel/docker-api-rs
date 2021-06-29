//! Create and manage user-defined networks that containers can be attached to.
//!
//! API Reference: <https://docs.docker.com/engine/api/v1.41/#tag/Network>

use std::{collections::HashMap, hash::Hash};

use hyper::Body;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use url::form_urlencoded;

use crate::{
    docker::Docker,
    errors::{Error, Result},
    transport::Payload,
};

#[derive(Debug)]
/// Interface for docker network
///
/// API Reference: <https://docs.docker.com/engine/api/v1.41/#tag/Network>
pub struct Networks<'docker> {
    docker: &'docker Docker,
}

impl<'docker> Networks<'docker> {
    /// Exports an interface for interacting with docker Networks
    pub fn new(docker: &'docker Docker) -> Self {
        Networks { docker }
    }

    /// List the docker networks on the current docker host
    ///
    /// API Reference: <https://docs.docker.com/engine/api/v1.41/#operation/NetworkList>
    pub async fn list(&self, opts: &NetworkListOptions) -> Result<Vec<NetworkInfo>> {
        let mut path = vec!["/networks".to_owned()];
        if let Some(query) = opts.serialize() {
            path.push(query);
        }
        self.docker.get_json(&path.join("?")).await
    }

    /// Returns a reference to a set of operations available to a specific network instance
    pub fn get<I>(&self, id: I) -> Network<'docker>
    where
        I: Into<String>,
    {
        Network::new(self.docker, id)
    }

    /// Create a new Network instance
    ///
    /// API Reference: <https://docs.docker.com/engine/api/v1.41/#operation/NetworkCreate>
    pub async fn create(&self, opts: &NetworkCreateOptions) -> Result<NetworkCreateInfo> {
        let body: Body = opts.serialize()?.into();
        let path = vec!["/networks/create".to_owned()];

        self.docker
            .post_json(&path.join("?"), Payload::Json(body))
            .await
    }
}

#[derive(Debug)]
/// Interface for accessing and manipulating a docker network
pub struct Network<'docker> {
    docker: &'docker Docker,
    id: String,
}

impl<'docker> Network<'docker> {
    /// Exports an interface exposing operations against a network instance
    pub fn new<S>(docker: &'docker Docker, id: S) -> Self
    where
        S: Into<String>,
    {
        Network {
            docker,
            id: id.into(),
        }
    }

    /// a getter for the Network id
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Inspects the current docker network instance's details
    ///
    /// API Reference: <https://docs.docker.com/engine/api/v1.41/#operation/NetworkInspect>
    pub async fn inspect(&self) -> Result<NetworkInfo> {
        self.docker
            .get_json(&format!("/networks/{}", self.id)[..])
            .await
    }

    /// Delete the network instance
    ///
    /// API Reference: <https://docs.docker.com/engine/api/v1.41/#operation/NetworkDelete>
    pub async fn delete(&self) -> Result<()> {
        self.docker
            .delete(&format!("/networks/{}", self.id)[..])
            .await?;
        Ok(())
    }

    /// Connect container to network
    ///
    /// API Reference: <https://docs.docker.com/engine/api/v1.41/#operation/NetworkConnect>
    pub async fn connect(&self, opts: &ContainerConnectionOptions) -> Result<()> {
        self.do_connection("connect", opts).await
    }

    /// Disconnect container to network
    ///
    /// API Reference: <https://docs.docker.com/engine/api/v1.41/#operation/NetworkDisconnect>
    pub async fn disconnect(&self, opts: &ContainerConnectionOptions) -> Result<()> {
        self.do_connection("disconnect", opts).await
    }

    async fn do_connection<S>(&self, segment: S, opts: &ContainerConnectionOptions) -> Result<()>
    where
        S: AsRef<str>,
    {
        let body: Body = opts.serialize()?.into();

        self.docker
            .post(
                &format!("/networks/{}/{}", self.id, segment.as_ref())[..],
                Payload::Json(body),
            )
            .await?;
        Ok(())
    }
}

/// Options for filtering networks list results
#[derive(Default, Debug)]
pub struct NetworkListOptions {
    params: HashMap<&'static str, String>,
}

impl NetworkListOptions {
    /// serialize options as a string. returns None if no options are defined
    pub fn serialize(&self) -> Option<String> {
        if self.params.is_empty() {
            None
        } else {
            Some(
                form_urlencoded::Serializer::new(String::new())
                    .extend_pairs(&self.params)
                    .finish(),
            )
        }
    }
}

/// Interface for creating new docker network
#[derive(Serialize, Debug)]
pub struct NetworkCreateOptions {
    params: HashMap<&'static str, Value>,
}

impl NetworkCreateOptions {
    /// return a new instance of a builder for options
    pub fn builder<N>(name: N) -> NetworkCreateOptionsBuilder
    where
        N: AsRef<str>,
    {
        NetworkCreateOptionsBuilder::new(name.as_ref())
    }

    /// serialize options as a string. returns None if no options are defined
    pub fn serialize(&self) -> Result<String> {
        serde_json::to_string(&self.params).map_err(Error::from)
    }
}

#[derive(Default)]
pub struct NetworkCreateOptionsBuilder {
    params: HashMap<&'static str, Value>,
}

impl NetworkCreateOptionsBuilder {
    pub(crate) fn new(name: &str) -> Self {
        let mut params = HashMap::new();
        params.insert("Name", json!(name));
        NetworkCreateOptionsBuilder { params }
    }

    impl_str_field!(driver: D => "Driver");

    impl_map_field!(labels: L => "Labels");

    pub fn build(&self) -> NetworkCreateOptions {
        NetworkCreateOptions {
            params: self.params.clone(),
        }
    }
}

/// Interface for connect container to network
#[derive(Serialize, Debug)]
pub struct ContainerConnectionOptions {
    params: HashMap<&'static str, Value>,
}

impl ContainerConnectionOptions {
    /// serialize options as a string. returns None if no options are defined
    pub fn serialize(&self) -> Result<String> {
        serde_json::to_string(&self.params).map_err(Error::from)
    }

    /// return a new instance of a builder for options
    pub fn builder<I>(container_id: I) -> ContainerConnectionOptionsBuilder
    where
        I: AsRef<str>,
    {
        ContainerConnectionOptionsBuilder::new(container_id.as_ref())
    }
}

#[derive(Default)]
pub struct ContainerConnectionOptionsBuilder {
    params: HashMap<&'static str, Value>,
}

impl ContainerConnectionOptionsBuilder {
    pub(crate) fn new(container_id: &str) -> Self {
        let mut params = HashMap::new();
        params.insert("Container", json!(container_id));
        ContainerConnectionOptionsBuilder { params }
    }

    pub fn aliases<A, S>(&mut self, aliases: A) -> &mut Self
    where
        A: IntoIterator<Item = S>,
        S: AsRef<str> + Serialize,
    {
        self.params.insert(
            "EndpointConfig",
            json!({ "Aliases": json!(aliases.into_iter().collect::<Vec<_>>()) }),
        );
        self
    }

    pub fn force(&mut self) -> &mut Self {
        self.params.insert("Force", json!(true));
        self
    }

    pub fn build(&self) -> ContainerConnectionOptions {
        ContainerConnectionOptions {
            params: self.params.clone(),
        }
    }
}

type PortDescription = HashMap<String, Option<Vec<HashMap<String, String>>>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NetworkSettings {
    pub bridge: String,
    pub gateway: String,
    #[serde(rename = "IPAddress")]
    pub ip_address: String,
    #[serde(rename = "IPPrefixLen")]
    pub ip_prefix_len: u64,
    pub mac_address: String,
    pub ports: Option<PortDescription>,
    pub networks: HashMap<String, NetworkEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NetworkEntry {
    #[serde(rename = "NetworkID")]
    pub network_id: String,
    #[serde(rename = "EndpointID")]
    pub endpoint_id: String,
    pub gateway: String,
    #[serde(rename = "IPAddress")]
    pub ip_address: String,
    #[serde(rename = "IPPrefixLen")]
    pub ip_prefix_len: u64,
    #[serde(rename = "IPv6Gateway")]
    pub ipv6_gateway: String,
    #[serde(rename = "GlobalIPv6Address")]
    pub global_ipv6_address: String,
    #[serde(rename = "GlobalIPv6PrefixLen")]
    pub global_ipv6_prefix_len: u64,
    pub mac_address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub rx_dropped: u64,
    pub rx_bytes: u64,
    pub rx_errors: u64,
    pub tx_packets: u64,
    pub tx_dropped: u64,
    pub rx_packets: u64,
    pub tx_errors: u64,
    pub tx_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Ipam {
    pub driver: String,
    pub config: Vec<HashMap<String, String>>,
    pub options: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NetworkDetails {
    pub name: String,
    pub id: String,
    pub scope: String,
    pub driver: String,
    #[serde(rename = "EnableIPv6")]
    pub enable_ipv6: bool,
    #[serde(rename = "IPAM")]
    pub ipam: Ipam,
    pub internal: bool,
    pub attachable: bool,
    pub containers: HashMap<String, NetworkContainerDetails>,
    pub options: Option<HashMap<String, String>>,
    pub labels: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NetworkContainerDetails {
    #[serde(rename = "EndpointID")]
    pub endpoint_id: String,
    pub mac_address: String,
    #[serde(rename = "IPv4Address")]
    pub ipv4_address: String,
    #[serde(rename = "IPv6Address")]
    pub ipv6_address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NetworkCreateInfo {
    pub id: String,
    pub warning: String,
}
