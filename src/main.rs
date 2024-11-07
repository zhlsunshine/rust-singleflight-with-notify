use std::collections::{HashMap, HashSet};
use std::env;
use std::net::IpAddr;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::Notify;

#[derive(Default, Debug, Clone)]
struct DnsResolver {
    // Map of resolved hostnames.
    resolved: Arc<RwLock<HashMap<String, ResolvedDns>>>,
    // Map of in-progress resolution requests.
    in_progress: Arc<Mutex<HashMap<String, Arc<Notify>>>>,
}

#[allow(dead_code)]
#[derive(Default, Debug, Clone)]
struct ResolvedDns {
    hostname: String,
    ips: HashSet<IpAddr>,
    initial_query: Option<std::time::Instant>,
    dns_refresh_rate: std::time::Duration,
}

impl DnsResolver {
    fn new() -> Self {
        Self {
            resolved: Arc::new(RwLock::new(HashMap::new())),
            in_progress: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn resolve_host(&self, hostname: String) -> Option<ResolvedDns> {
        let (notify, is_first) = self.get_or_create_notify(&hostname);

        if is_first {
            println!("first get into the host name resolving task!");
            // Perform the resolution only if it's the first request.
            self.resolve_on_demand_dns(&hostname).await;
            // Notify all waiters after the DNS resolving task completed.
            notify.notify_waiters();

            // The resolution is complete. We can remove the in-progress notify object.
            self.in_progress.lock().unwrap().remove(&hostname);
        } else {
            // Wait for the in-progress resolution to complete.
            notify.notified().await;
        }

        // Serve from the local cache after resolution completes.
        self.find_resolved_host(&hostname)
    }

    fn get_or_create_notify(&self, hostname: &String) -> (Arc<Notify>, bool) {
        let mut in_progress = self.in_progress.lock().unwrap();
        match in_progress.get(hostname) {
            Some(notify) => (notify.clone(), false),
            None => {
                let notify = Arc::new(Notify::new());
                in_progress.insert(hostname.clone(), notify.clone());
                (notify, true)
            }
        }
    }

    fn find_resolved_host(&self, hostname: &String) -> Option<ResolvedDns> {
        self.resolved
            .read()
            .unwrap()
            .get(hostname)
            .filter(|rdns| {
                rdns.initial_query.is_some()
                    && rdns.initial_query.unwrap().elapsed() < rdns.dns_refresh_rate
            })
            .cloned()
    }

    async fn resolve_on_demand_dns(&self, hostname: &String) {
        // Simulated DNS resolution delay
        thread::sleep(Duration::from_secs(2));
        // Here you would perform the actual DNS resolution
        // and update the resolved map with the result.
        let mut resolved_map = self.resolved.write().unwrap();
        resolved_map.insert(
            hostname.clone(),
            ResolvedDns {
                hostname: hostname.clone(),
                ips: HashSet::new(), // Placeholder for resolved IPs
                initial_query: Some(std::time::Instant::now()),
                dns_refresh_rate: Duration::from_secs(60), // Example refresh rate
            },
        );
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    // Find the concurrency parameter
    let concurrency_value = args.iter().position(|arg| arg == "--concurrency")
        .and_then(|index| args.get(index + 1))
        .expect("Concurrency parameter not provided");

    let concurrency: i32 = concurrency_value.parse()
         .expect("Invalid concurrency parameter value");

    let resolver = DnsResolver::new();

    let start = Instant::now();
    // Spawn multiple tasks to simulate concurrent DNS resolution requests.
    let tasks = (0..concurrency)
        .map(|_i| {
            let resolver = resolver.clone();
            tokio::spawn(async move {
                let hostname = format!("example.com");
                let _result = resolver.resolve_host(hostname).await;
            })
        })
        .collect::<Vec<_>>();

    // Wait for all tasks to complete.
    for task in tasks {
        let _ = task.await;
    }
    let end = Instant::now();
    let duration = end - start;
    println!("Time taken: {:?}", duration);
}
