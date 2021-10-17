// Copyright 2021 Oxide Computer Company
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use rift_protocol::{Level, net::Ipv6Prefix};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, JsonSchema)]
pub struct Config {
    pub id: u64,
    pub level: Level,
    pub prefix: Option<Ipv6Prefix>,
}
