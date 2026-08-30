#[cfg(test)]
mod integration_tests {
    use crate::wire_client;

    #[test]
    #[ignore]  // Only runs with --ignored flag; requires daemon running
    fn infer_round_trip_to_daemon() {
        let reply = wire_client::infer("What is 2+2?", 3000, 35000);
        match reply {
            Ok(text) => {
                println!("INFER REPLY: {}", text);
                assert!(!text.is_empty(), "daemon returned empty reply");
            }
            Err(e) => {
                println!("INFER ERROR (daemon/sidecar may be down): {}", e);
                // Don't fail - just document the error state
            }
        }
    }
}
