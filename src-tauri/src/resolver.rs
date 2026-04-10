use std::net::{SocketAddr, ToSocketAddrs};
use reqwest::dns::{Addrs, Name, Resolve, Resolving};

#[derive(Clone, Default)]
pub struct CustomHickoryResolver;

impl Resolve for CustomHickoryResolver {
    fn resolve(&self, name: Name) -> Resolving {
        Box::pin(async move {
            let host_port = format!("{}:0", name.as_str());
            
            // We use tokio::task::spawn_blocking because ToSocketAddrs is blocking
            let addrs = tokio::task::spawn_blocking(move || {
                let addrs = host_port.to_socket_addrs()?;
                // Filter out IPv6 addresses to avoid TCP timeout on Android
                let ipv4_only: Vec<SocketAddr> = addrs.filter(|a| a.is_ipv4()).collect();
                
                if ipv4_only.is_empty() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "No IPv4 addresses found",
                    ));
                }
                
                Ok(ipv4_only)
            })
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

            let addrs: Addrs = Box::new(addrs.into_iter());
            Ok(addrs)
        })
    }
}
