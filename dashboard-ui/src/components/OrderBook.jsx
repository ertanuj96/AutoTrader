import { useApi } from '../hooks/useApi';

export default function OrderBook() {
  const { data } = useApi('/orders?limit=20', 2000);
  const orders = data?.orders || [];

  const formatTime = (ms) => {
    if (!ms) return '--';
    const d = new Date(Number(ms));
    return d.toLocaleTimeString('en-IN', { hour12: false });
  };

  return (
    <div className="card orderbook-card">
      <div className="card-header">
        <span className="card-title">📋 Recent Orders</span>
        <span style={{ fontFamily: 'var(--font-mono)', fontSize: '0.75rem', color: 'var(--text-muted)' }}>
          {orders.length} orders
        </span>
      </div>
      <div className="card-body" style={{ padding: 0, maxHeight: 360, overflowY: 'auto' }}>
        {orders.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon">📋</div>
            <div>No orders yet</div>
          </div>
        ) : (
          orders.map((order, i) => (
            <div className="order-item" key={i}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                <span className={`order-side ${order.side?.toLowerCase()}`}>
                  {order.side}
                </span>
                <div>
                  <div style={{ color: 'var(--text-primary)', fontWeight: 500, fontSize: '0.8rem' }}>
                    {order.instrument}
                  </div>
                  <div style={{ color: 'var(--text-muted)', fontSize: '0.65rem' }}>
                    {order.quantity} × {order.order_type} @ ₹{Number(order.fill_price || order.limit_price || 0).toFixed(2)}
                  </div>
                </div>
              </div>
              <div style={{ textAlign: 'right' }}>
                <span className={`order-status ${order.status?.toLowerCase()}`}>
                  {order.status}
                </span>
                <div style={{ color: 'var(--text-muted)', fontSize: '0.6rem', marginTop: 2 }}>
                  {formatTime(order.timestamp_ms)}
                </div>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
