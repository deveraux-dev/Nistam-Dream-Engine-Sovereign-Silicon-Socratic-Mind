//! HTTP gatekeeper: 4KB payload cap, 2000ms timeout, credit burn, blank error bodies.
//! Sync TcpListener + thread-per-connection on vixio-v3 reactor foundation.
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Instant;

/// Credit ledger and atomic burn.
pub mod credit;
/// Deadline reactor stub for wall-clock timeout enforcement.
pub mod deadline;
/// Request size caps and JSON shape validation.
pub mod limits;
/// HTTP response writer.
pub mod respond;

pub use credit::CreditLedger;
pub use deadline::DeadlineReactor;
pub use limits::RequestError;

/// HTTP gatekeeper: accepts connections, enforces credit/cap/timeout per request.
pub struct HttpGate {
    listener: TcpListener,
    credits: Arc<CreditLedger>,
    reactor: Arc<DeadlineReactor>,
}

impl HttpGate {
    /// Bind to addr, starting with initial_credits balance and deadline reactor on 1ms ticks.
    pub fn bind(addr: &str, initial_credits: u64) -> std::io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        let credits = Arc::new(CreditLedger::new(initial_credits));
        let reactor = Arc::new(DeadlineReactor::new());
        Ok(Self {
            listener,
            credits,
            reactor,
        })
    }

    /// Accept loop: spawn thread per connection, enforce limits/deadline/credit, proxy upstream.
    pub fn run(&self) -> std::io::Result<()> {
        for stream in self.listener.incoming() {
            let stream = stream?;
            let credits = Arc::clone(&self.credits);
            let reactor = Arc::clone(&self.reactor);

            std::thread::spawn(move || {
                if let Err(e) = Self::handle_connection(stream, credits, reactor) {
                    eprintln!("Connection error: {:?}", e);
                }
            });
        }
        Ok(())
    }

    fn handle_connection(
        mut stream: TcpStream,
        credits: Arc<CreditLedger>,
        _reactor: Arc<DeadlineReactor>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn_start = Instant::now();

        // 1. Burn credit or reject with 402.
        if !credits.try_burn() {
            respond::write_status(&mut stream, 402, "Payment Required")?;
            return Ok(());
        }

        // 2. Read capped to 4096 bytes or reject with 413.
        let body = match limits::read_capped(&mut stream, 4096) {
            Ok(b) => b,
            Err(RequestError::TooLarge) => {
                respond::write_status(&mut stream, 413, "Payload Too Large")?;
                return Ok(());
            }
            Err(_) => {
                respond::write_status(&mut stream, 400, "Bad Request")?;
                return Ok(());
            }
        };

        // 3. Probe JSON shape (balanced braces, non-empty).
        if !limits::json_shape_ok(&body) {
            respond::write_status(&mut stream, 400, "Bad Request")?;
            return Ok(());
        }

        // 4. Extract query from JSON body (stub: the full body for now).
        let query = String::from_utf8_lossy(&body).to_string();

        // 5. Check deadline: hard 2000ms wall-clock timeout.
        let elapsed_ms = conn_start.elapsed().as_millis() as u64;
        let deadline_ms = 2000_u64;
        if elapsed_ms >= deadline_ms {
            respond::write_status(&mut stream, 504, "Gateway Timeout")?;
            return Ok(());
        }

        // 6. Forward to upstream (127.0.0.1:13017) via frame protocol.
        // For now, forward the query text verbatim; real JSON extraction is a follow-up.
        match Self::proxy_upstream(&query, deadline_ms - elapsed_ms) {
            Ok(reply) => {
                respond::write_status(&mut stream, 200, "OK")?;
                stream.write_all(reply.as_bytes())?;
            }
            Err(_) => {
                respond::write_status(&mut stream, 504, "Gateway Timeout")?;
            }
        }

        Ok(())
    }

    fn proxy_upstream(query: &str, budget_ms: u64) -> Result<String, std::io::Error> {
        use std::time::Duration;
        let timeout = Duration::from_millis(budget_ms.max(1));
        let mut stream = std::net::TcpStream::connect_timeout(
            &"127.0.0.1:13017".parse().expect("valid addr"),
            timeout,
        )?;
        stream.set_read_timeout(Some(timeout))?;

        // Write frame: u32-BE length + payload.
        let len = query.len() as u32;
        stream.write_all(&len.to_be_bytes())?;
        stream.write_all(query.as_bytes())?;

        // Read frame: u32-BE length + payload.
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes)?;
        let len = u32::from_be_bytes(len_bytes) as usize;
        if len > 1_000_000 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "frame too large",
            ));
        }
        let mut buf = vec![0u8; len];
        stream.read_exact(&mut buf)?;
        String::from_utf8(buf)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "not UTF-8"))
    }
}
