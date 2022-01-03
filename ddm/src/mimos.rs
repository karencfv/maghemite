use std::io::Result;
use std::sync::Arc;

use tokio::sync::mpsc::{Sender, Receiver, channel};
use tokio::{spawn, select};
use tokio::sync::Mutex;

use crate::platform;
use crate::port::Port;
use crate::rdp::RdpMessage;
use crate::protocol::{DdmMessage, PeerMessage, RouterKind};
use crate::router::{Route, Router};
use crate::config::Config;
pub struct RdpChannel {
    pub rx: Receiver<RdpMessage>,
    pub tx: Sender<RdpMessage>,
}

pub struct DdmChannel {
    pub rx: Receiver<DdmMessage>,
    pub tx: Sender<DdmMessage>,
}

pub struct PeerChannel {
    pub rx: Receiver<PeerMessage>,
    pub tx: Sender<PeerMessage>,
}

#[derive(Clone)]
pub struct Neighbor {
    pub rdp_ch: Arc::<Mutex::<RdpChannel>>,
    pub peer_ch: Arc::<Mutex::<PeerChannel>>,
    pub ddm_ch: Arc::<Mutex::<DdmChannel>>,
}

pub struct Node {
    pub name: String,
    pub platform: Arc::<Mutex::<Platform>>,
    pub router: Arc::<Router>,
}

impl Node {
    pub fn new(name: String, kind: RouterKind) -> Self {
        Node{
            name: name.clone(),
            platform: Arc::new(Mutex::new(Platform::new())),
            router: Arc::new(Router::new(name, kind)),
        }
    }
    
    pub fn run(&self, port: u16, log: slog::Logger) -> Result<()> {
        //self.router.run(self.platform.clone(), Config{port}, log)
        Router::run(
            self.router.clone(), self.platform.clone(), Config{port}, log)
    }
}

#[derive(Clone)]
pub struct Platform {
    neighbors: Vec<Neighbor>,
}

impl Platform {
    pub fn new() -> Self {
        Platform{
            neighbors: Vec::new(),
        }
    }
}

pub async fn connect(a: &mut Node, b: &mut Node) {

    let (rdp_tx_ab, rdp_rx_ab) = channel(0x20);
    let (rdp_tx_ba, rdp_rx_ba) = channel(0x20);

    let (ddm_tx_ab, ddm_rx_ab) = channel(0x200);
    let (ddm_tx_ba, ddm_rx_ba) = channel(0x200);

    let (peer_tx_ab, peer_rx_ab) = channel(0x20);
    let (peer_tx_ba, peer_rx_ba) = channel(0x20);

    a.platform.lock().await.neighbors.push(Neighbor{
        rdp_ch: Arc::new(Mutex::new(RdpChannel{
            rx: rdp_rx_ba,
            tx: rdp_tx_ab,
        })),
        ddm_ch: Arc::new(Mutex::new(DdmChannel{
            rx: ddm_rx_ba,
            tx: ddm_tx_ab,
        })),
        peer_ch: Arc::new(Mutex::new(PeerChannel{
            rx: peer_rx_ba,
            tx: peer_tx_ab,
        }))
    });

    b.platform.lock().await.neighbors.push(Neighbor{
        rdp_ch: Arc::new(Mutex::new(RdpChannel{
            rx: rdp_rx_ab,
            tx: rdp_tx_ba,
        })),
        ddm_ch: Arc::new(Mutex::new(DdmChannel{
            rx: ddm_rx_ab,
            tx: ddm_tx_ba,
        })),
        peer_ch: Arc::new(Mutex::new(PeerChannel{
            rx: peer_rx_ab,
            tx: peer_tx_ba,
        }))
    });

}

impl platform::Ports for Platform {
    fn ports(&self) -> Result<Vec<Port>> {
        let mut result = Vec::new();
        for (index, _) in self.neighbors.iter().enumerate() {
            result.push(Port{index})
        }
        Ok(result)
    }
}

impl platform::Rdp for Platform {
    fn rdp_channel(&self, p: Port)
    -> Result<(Sender<RdpMessage>, Receiver<RdpMessage>)> {

        let (itx, irx) = channel(0x20);
        let (etx, mut erx) = channel(0x20);

        let rdp = self.neighbors[p.index].rdp_ch.clone();

        spawn(async move{
            //let mut n = nbr.lock().await;
            let mut rdp = rdp.lock().await;
            loop {
                select!(
                    msg = rdp.rx.recv() => {
                        match msg {
                            Some(m) => {
                                match itx.send(m).await {
                                    Ok(()) => {}
                                    Err(e) => {
                                        println!("rdp ingress send: {}", e)
                                    }
                                }
                            }
                            None => {}
                        };
                    }
                    msg = erx.recv() => {
                        match msg {
                            Some(m) => {
                                match rdp.tx.send(m).await {
                                    Ok(()) => {}
                                    Err(e) => {
                                        println!("rdp egress send: {}", e)
                                    }
                                }
                            }
                            None => {}
                        }
                    }
                );
            }
        });

        Ok((etx, irx))
    }
}

impl platform::Ddm for Platform {

    fn peer_channel(&self, p: Port) 
    -> Result<(Sender<PeerMessage>, Receiver<PeerMessage>)> {
        let (itx, irx) = channel(0x20);
        let (etx, mut erx) = channel(0x20);

        let pc = self.neighbors[p.index].peer_ch.clone();

        spawn(async move{
            loop {
                let mut pc = pc.lock().await;
                select!(
                    msg = pc.rx.recv() => {
                        match msg {
                            Some(m) => {
                                match itx.send(m).await {
                                    Ok(()) => {}
                                    Err(e) => {
                                        println!("peer ingress send: {}", e)
                                    }
                                }
                            }
                            None => {}
                        };
                    }
                    msg = erx.recv() => {
                        match msg {
                            Some(m) => {
                                match pc.tx.send(m).await {
                                    Ok(()) => {}
                                    Err(e) => {
                                        println!("peer egress send: {}", e)
                                    }
                                }
                            }
                            None => {}
                        }
                    }
                );
            }
        });

        Ok((etx, irx))
    }

    fn ddm_channel(&self, p: Port) 
    -> Result<(Sender<DdmMessage>, Receiver<DdmMessage>)> {
        let (itx, irx) = channel(0x20);
        let (etx, mut erx) = channel(0x20);

        let ddm = self.neighbors[p.index].ddm_ch.clone();

        spawn(async move{
            let mut ddm = ddm.lock().await;
            loop {
                select!(
                    msg = ddm.rx.recv() => {
                        match msg {
                            Some(m) => {
                                match itx.send(m).await {
                                    Ok(()) => {}
                                    Err(e) => {
                                        println!("ddm ingress send: {}", e)
                                    }
                                }
                            }
                            None => {}
                        };
                    }
                    msg = erx.recv() => {
                        match msg {
                            Some(m) => {
                                match ddm.tx.send(m).await {
                                    Ok(()) => {}
                                    Err(e) => {
                                        println!("ddm egress send: {}", e)
                                    }
                                }
                            }
                            None => {}
                        }
                    }
                );
            }
        });

        Ok((etx, irx))
    }
}

impl platform::Router for Platform {
    fn get_routes(&self) -> Result<Vec<Route>> {
        todo!();
    }

    fn set_route(&self, _r: Route) -> Result<()> {
        todo!();
    }

    fn delete_route(&self, _r: Route) -> Result<()> {
        todo!();
    }
}

#[cfg(test)]
mod test {
    use crate::mimos;
    use crate::port::Port;
    use crate::platform::{Rdp, Ddm};
    use crate::rdp::RdpMessage;
    use crate::protocol::{DdmMessage, DdmPrefix, RouterKind};
    use crate::net::Ipv6Prefix;

    use std::str::FromStr;
    use std::collections::HashSet;

    #[tokio::test]
    async fn mimos_2_router_msg() -> anyhow::Result<()> {

        // topology
        let mut a = mimos::Node::new("a".into(), RouterKind::Server);
        let mut b = mimos::Node::new("a".into(), RouterKind::Server);
        mimos::connect(&mut a, &mut b).await;

        // get RDP channel
        let (a_rdp_tx, mut a_rdp_rx) =
            a.platform.lock().await.rdp_channel(Port{index: 0}).unwrap();
        let (b_rdp_tx, mut b_rdp_rx) = 
            b.platform.lock().await.rdp_channel(Port{index: 0}).unwrap();

        // get DDM channel
        let (a_ddm_tx, mut a_ddm_rx) = 
            a.platform.lock().await.ddm_channel(Port{index: 0}).unwrap();
        let (b_ddm_tx, mut b_ddm_rx) =
            b.platform.lock().await.ddm_channel(Port{index: 0}).unwrap();

        // send some RDP messages
        a_rdp_tx.send(RdpMessage{content: "rdp test 1".into()}).await?;
        b_rdp_tx.send(RdpMessage{content: "rdp test 2".into()}).await?;

        // receive RDP messages
        let msg = a_rdp_rx.recv().await;
        assert_eq!(msg, Some(RdpMessage{content: "rdp test 2".into()}));

        let msg = b_rdp_rx.recv().await;
        assert_eq!(msg, Some(RdpMessage{content: "rdp test 1".into()}));

        // send some DDM messages
        let mut prefixes = HashSet::new();
        prefixes.insert(Ipv6Prefix::from_str("fd00::1701/64")?);
        let a_to_b = DdmMessage::Prefix(DdmPrefix{
            origin: "a".to_string(),
            serial: 0,
            prefixes,
        });
        a_ddm_tx.send(a_to_b.clone()).await?;

        let mut prefixes = HashSet::new();
        prefixes.insert(Ipv6Prefix::from_str("fd00::1702/64")?);
        let b_to_a = DdmMessage::Prefix(DdmPrefix{
            origin: "b".to_string(),
            serial: 0,
            prefixes,
        });
        b_ddm_tx.send(b_to_a.clone()).await?;

        // receive DDM messages
        let msg = a_ddm_rx.recv().await;
        assert_eq!(msg.unwrap(), b_to_a);

        let msg = b_ddm_rx.recv().await;
        assert_eq!(msg.unwrap(), a_to_b);

        Ok(())

    }
}
