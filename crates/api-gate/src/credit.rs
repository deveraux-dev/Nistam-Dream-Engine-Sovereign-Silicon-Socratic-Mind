/// Per-request credit ledger: atomic CAS-decrement, zero-balance rejects all.
use std::sync::atomic::{AtomicU64, Ordering};

/// Thread-safe credit balance: try_burn deducts 1 or returns false at zero.
pub struct CreditLedger(pub AtomicU64);

impl CreditLedger {
    /// Create a new ledger with initial credit balance.
    pub fn new(initial: u64) -> Self {
        Self(AtomicU64::new(initial))
    }

    /// Attempt to burn one credit. Returns false (deny) if balance == 0 before the burn.
    pub fn try_burn(&self) -> bool {
        let mut current = self.0.load(Ordering::SeqCst);
        loop {
            if current == 0 {
                return false;
            }
            match self.0.compare_exchange(
                current,
                current - 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return true,
                Err(actual) => current = actual,
            }
        }
    }

    /// Read current balance (eventual consistency, not atomic with burns).
    pub fn balance(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credit_ledger_burns_to_zero() {
        let ledger = CreditLedger::new(3);
        assert!(ledger.try_burn());
        assert_eq!(ledger.balance(), 2);
        assert!(ledger.try_burn());
        assert_eq!(ledger.balance(), 1);
        assert!(ledger.try_burn());
        assert_eq!(ledger.balance(), 0);
        assert!(!ledger.try_burn());
        assert_eq!(ledger.balance(), 0);
    }

    #[test]
    fn credit_ledger_refuses_below_zero() {
        let ledger = CreditLedger::new(1);
        assert!(ledger.try_burn());
        assert!(!ledger.try_burn());
        assert!(!ledger.try_burn());
    }

    #[test]
    fn credit_ledger_zero_from_start() {
        let ledger = CreditLedger::new(0);
        assert!(!ledger.try_burn());
    }
}
