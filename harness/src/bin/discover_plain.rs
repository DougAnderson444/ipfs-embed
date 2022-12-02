#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use anyhow::Context;
    use futures::FutureExt;
    use harness::{MultiaddrExt, MyFutureExt, NetsimExt, Role};
    use ipfs_embed_cli::{Command, Event};
    use maplit::hashmap;
    use std::time::Instant;

    harness::build_bin()?;

    harness::run_netsim(|mut sim, opts, _net, _tmp| {
        async move {
            let providers = sim.role(&opts, Role::Provider);
            let consumers = sim.role(&opts, Role::Consumer);

            let started = Instant::now();
            for id in providers.keys().chain(consumers.keys()) {
                let m = sim.machine(*id);
                m.select(|e| matches!(e, Event::NewListenAddr(a) if !a.is_loopback()).then(|| ()))
                    .deadline(started, 5)
                    .await
                    .unwrap();
            }

            for id in consumers.keys() {
                let m = sim.machine(*id);
                for (peer, addr) in providers.values() {
                    m.send(Command::AddAddress(*peer, addr.clone()));
                    m.send(Command::Dial(*peer));
                }
            }

            let started = Instant::now();
            for id in consumers.keys() {
                let m = sim.machine(*id);
                for (peer, addr) in providers.values() {
                    m.select(|e| {
                        matches!(e, Event::PeerInfo(p, i)
                        if p == peer && i.addresses == hashmap!(addr.clone() => "Dial".to_owned())
                    )
                    .then(|| ())
                    })
                    .deadline(started, 5)
                    .await
                    .unwrap();
                }
            }

            let expected = if opts.disable_port_reuse {
                "Dial"
            } else {
                "Candidate"
            };
            for id in providers.keys() {
                let m = sim.machine(*id);
                for (peer, addr) in consumers.values() {
                    m.select(|e| {
                        matches!(e, Event::PeerInfo(p, i)
                        if p == peer && i.addresses == hashmap!(addr.clone() => expected.to_owned())
                    )
                    .then(|| ())
                    })
                    .deadline(started, 5)
                    .await
                    .unwrap();
                }
            }

            Ok(())
        }
        .timeout(120)
        .map(|r| r.unwrap_or_else(|e| Err(e.into())))
    })
    .context("netsim")
}

#[cfg(not(target_os = "linux"))]
fn main() {}
