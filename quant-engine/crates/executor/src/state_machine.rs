//! Idempotent order state machine for tracking order lifecycle.

use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

// ─── Order status variants ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum OrderStatus {
    New,
    Submitted,
    Acknowledged { broker_order_id: String },
    PartiallyFilled { filled_qty: u32, avg_price: f64, broker_order_id: String },
    Filled          { filled_qty: u32, avg_price: f64, broker_order_id: String },
    Cancelled       { reason: String },
    Rejected        { reason: String },
}

// ─── Order record ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct OrderRecord {
    pub order_id:     Uuid,
    pub signal_id:    Uuid,
    pub symbol:       String,
    pub exchange:     String,
    pub side:         String,
    pub requested_qty: u32,
    pub status:       OrderStatus,
    pub created_at:   DateTime<Utc>,
    pub updated_at:   DateTime<Utc>,
}

// ─── State machine ────────────────────────────────────────────────────────────

pub struct OrderStateMachine {
    orders: HashMap<Uuid, OrderRecord>,
}

impl OrderStateMachine {
    pub fn new() -> Self {
        Self { orders: HashMap::new() }
    }

    /// Insert a new order (must start in New or Submitted status).
    pub fn submit(&mut self, record: OrderRecord) {
        self.orders.insert(record.order_id, record);
    }

    /// Transition: Submitted → Acknowledged.
    pub fn acknowledge(
        &mut self,
        order_id: Uuid,
        broker_order_id: String,
    ) -> Result<(), String> {
        let rec = self.get_mut_or_err(order_id)?;
        match &rec.status {
            OrderStatus::Submitted => {
                rec.status = OrderStatus::Acknowledged { broker_order_id };
                rec.updated_at = Utc::now();
                Ok(())
            }
            s => Err(format!("Cannot transition from {:?} to Acknowledged", s)),
        }
    }

    /// Transition: Submitted | Acknowledged | PartiallyFilled → Filled.
    pub fn fill(
        &mut self,
        order_id: Uuid,
        qty: u32,
        avg_price: f64,
        broker_id: String,
    ) -> Result<(), String> {
        let rec = self.get_mut_or_err(order_id)?;
        match &rec.status {
            OrderStatus::Submitted
            | OrderStatus::Acknowledged { .. }
            | OrderStatus::PartiallyFilled { .. } => {
                rec.status = OrderStatus::Filled {
                    filled_qty: qty,
                    avg_price,
                    broker_order_id: broker_id,
                };
                rec.updated_at = Utc::now();
                Ok(())
            }
            s => Err(format!("Cannot transition from {:?} to Filled", s)),
        }
    }

    /// Transition: Submitted | Acknowledged | PartiallyFilled → PartiallyFilled.
    pub fn partial_fill(
        &mut self,
        order_id: Uuid,
        qty: u32,
        avg_price: f64,
        broker_id: String,
    ) -> Result<(), String> {
        let rec = self.get_mut_or_err(order_id)?;
        match &rec.status {
            OrderStatus::Submitted
            | OrderStatus::Acknowledged { .. }
            | OrderStatus::PartiallyFilled { .. } => {
                rec.status = OrderStatus::PartiallyFilled {
                    filled_qty: qty,
                    avg_price,
                    broker_order_id: broker_id,
                };
                rec.updated_at = Utc::now();
                Ok(())
            }
            s => Err(format!("Cannot transition from {:?} to PartiallyFilled", s)),
        }
    }

    /// Transition: New | Submitted | Acknowledged | PartiallyFilled → Cancelled.
    pub fn cancel(&mut self, order_id: Uuid, reason: String) -> Result<(), String> {
        let rec = self.get_mut_or_err(order_id)?;
        match &rec.status {
            OrderStatus::New
            | OrderStatus::Submitted
            | OrderStatus::Acknowledged { .. }
            | OrderStatus::PartiallyFilled { .. } => {
                rec.status = OrderStatus::Cancelled { reason };
                rec.updated_at = Utc::now();
                Ok(())
            }
            s => Err(format!("Cannot transition from {:?} to Cancelled", s)),
        }
    }

    /// Transition: New | Submitted → Rejected.
    pub fn reject(&mut self, order_id: Uuid, reason: String) -> Result<(), String> {
        let rec = self.get_mut_or_err(order_id)?;
        match &rec.status {
            OrderStatus::New | OrderStatus::Submitted => {
                rec.status = OrderStatus::Rejected { reason };
                rec.updated_at = Utc::now();
                Ok(())
            }
            s => Err(format!("Cannot transition from {:?} to Rejected", s)),
        }
    }

    // ── Reads ─────────────────────────────────────────────────────────────────

    pub fn get(&self, order_id: &Uuid) -> Option<&OrderRecord> {
        self.orders.get(order_id)
    }

    /// Orders that are not yet in a terminal state.
    pub fn active_orders(&self) -> Vec<&OrderRecord> {
        self.orders.values().filter(|r| !Self::is_terminal(&r.status)).collect()
    }

    /// Orders that are in a terminal state (Filled, Cancelled, Rejected).
    pub fn completed_orders(&self) -> Vec<&OrderRecord> {
        self.orders.values().filter(|r| Self::is_terminal(&r.status)).collect()
    }

    pub fn len(&self) -> usize { self.orders.len() }

    pub fn is_empty(&self) -> bool { self.orders.is_empty() }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn is_terminal(status: &OrderStatus) -> bool {
        matches!(
            status,
            OrderStatus::Filled { .. } | OrderStatus::Cancelled { .. } | OrderStatus::Rejected { .. }
        )
    }

    fn get_mut_or_err(&mut self, order_id: Uuid) -> Result<&mut OrderRecord, String> {
        self.orders
            .get_mut(&order_id)
            .ok_or_else(|| format!("Order {order_id} not found"))
    }
}

impl Default for OrderStateMachine {
    fn default() -> Self { Self::new() }
}

// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn new_order(id: Uuid) -> OrderRecord {
        OrderRecord {
            order_id: id,
            signal_id: Uuid::new_v4(),
            symbol: "NIFTY".into(),
            exchange: "NSE".into(),
            side: "BUY".into(),
            requested_qty: 5,
            status: OrderStatus::Submitted,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn happy_path_submit_acknowledge_fill() {
        let mut sm = OrderStateMachine::new();
        let id = Uuid::new_v4();
        sm.submit(new_order(id));

        sm.acknowledge(id, "BRK-001".into()).unwrap();
        assert!(matches!(sm.get(&id).unwrap().status, OrderStatus::Acknowledged { .. }));

        sm.fill(id, 5, 18000.0, "BRK-001".into()).unwrap();
        assert!(matches!(sm.get(&id).unwrap().status, OrderStatus::Filled { .. }));

        assert_eq!(sm.active_orders().len(), 0);
        assert_eq!(sm.completed_orders().len(), 1);
    }

    #[test]
    fn partial_fill_then_full_fill() {
        let mut sm = OrderStateMachine::new();
        let id = Uuid::new_v4();
        sm.submit(new_order(id));

        sm.partial_fill(id, 2, 18000.0, "BRK-002".into()).unwrap();
        assert!(matches!(sm.get(&id).unwrap().status, OrderStatus::PartiallyFilled { .. }));

        sm.fill(id, 5, 18001.0, "BRK-002".into()).unwrap();
        assert!(matches!(sm.get(&id).unwrap().status, OrderStatus::Filled { .. }));
    }

    #[test]
    fn cancel_from_submitted() {
        let mut sm = OrderStateMachine::new();
        let id = Uuid::new_v4();
        sm.submit(new_order(id));
        sm.cancel(id, "user cancelled".into()).unwrap();
        assert!(matches!(sm.get(&id).unwrap().status, OrderStatus::Cancelled { .. }));
    }

    #[test]
    fn reject_from_new() {
        let mut sm = OrderStateMachine::new();
        let id = Uuid::new_v4();
        let mut rec = new_order(id);
        rec.status = OrderStatus::New;
        sm.submit(rec);
        sm.reject(id, "insufficient margin".into()).unwrap();
        assert!(matches!(sm.get(&id).unwrap().status, OrderStatus::Rejected { .. }));
    }

    #[test]
    fn invalid_acknowledge_from_new_returns_err() {
        let mut sm = OrderStateMachine::new();
        let id = Uuid::new_v4();
        let mut rec = new_order(id);
        rec.status = OrderStatus::New;
        sm.submit(rec);
        let err = sm.acknowledge(id, "BRK".into()).unwrap_err();
        assert!(err.contains("Cannot transition"), "err = {err}");
    }

    #[test]
    fn invalid_fill_from_filled_returns_err() {
        let mut sm = OrderStateMachine::new();
        let id = Uuid::new_v4();
        sm.submit(new_order(id));
        sm.fill(id, 5, 18000.0, "BRK".into()).unwrap();
        let err = sm.fill(id, 5, 18001.0, "BRK".into()).unwrap_err();
        assert!(err.contains("Cannot transition"), "err = {err}");
    }

    #[test]
    fn cancel_from_filled_returns_err() {
        let mut sm = OrderStateMachine::new();
        let id = Uuid::new_v4();
        sm.submit(new_order(id));
        sm.fill(id, 5, 18000.0, "BRK".into()).unwrap();
        let err = sm.cancel(id, "oops".into()).unwrap_err();
        assert!(err.contains("Cannot transition"), "err = {err}");
    }

    #[test]
    fn reject_from_acknowledged_returns_err() {
        let mut sm = OrderStateMachine::new();
        let id = Uuid::new_v4();
        sm.submit(new_order(id));
        sm.acknowledge(id, "BRK".into()).unwrap();
        let err = sm.reject(id, "bad".into()).unwrap_err();
        assert!(err.contains("Cannot transition"), "err = {err}");
    }

    #[test]
    fn len_tracks_orders() {
        let mut sm = OrderStateMachine::new();
        assert_eq!(sm.len(), 0);
        for _ in 0..3 { sm.submit(new_order(Uuid::new_v4())); }
        assert_eq!(sm.len(), 3);
    }
}
