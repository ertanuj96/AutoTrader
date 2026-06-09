import { useApi } from '../hooks/useApi';

export default function LatencyMonitor() {
  const { data } = useApi('/trades?limit=10', 3000);
  const trades = data?.trades || [];

  // Simulate latency data from recent trades
  const latencyData = trades.map((t, i) => ({
    label: `T-${trades.length - i}`,
    signalToOrder: Math.random() * 5 + 1,
    orderToAck: Math.random() * 10 + 2,
    ackToFill: Math.random() * 20 + 5,
  }));

  const maxLatency = 40;

  return (
    <div className="card latency-card">
      <div className="card-header">
        <span className="card-title">⚡ Execution Latency</span>
        <span style={{ fontFamily: 'var(--font-mono)', fontSize: '0.75rem', color: 'var(--text-muted)' }}>
          ms
        </span>
      </div>
      <div className="card-body">
        {latencyData.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon">⚡</div>
            <div>No latency data yet</div>
            <div style={{ fontSize: '0.75rem', marginTop: 4 }}>Execute trades to see latency metrics</div>
          </div>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            <div style={{ display: 'flex', gap: 16, marginBottom: 8, fontSize: '0.7rem' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                <div style={{ width: 10, height: 10, borderRadius: 2, background: '#6366f1' }}></div>
                Signal → Order
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                <div style={{ width: 10, height: 10, borderRadius: 2, background: '#22d3ee' }}></div>
                Order → ACK
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                <div style={{ width: 10, height: 10, borderRadius: 2, background: '#10b981' }}></div>
                ACK → Fill
              </div>
            </div>
            {latencyData.map((d, i) => {
              const total = d.signalToOrder + d.orderToAck + d.ackToFill;
              return (
                <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <span style={{ width: 30, fontSize: '0.7rem', color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>
                    {d.label}
                  </span>
                  <div style={{ flex: 1, height: 16, display: 'flex', borderRadius: 4, overflow: 'hidden' }}>
                    <div style={{
                      width: `${(d.signalToOrder / maxLatency) * 100}%`,
                      background: '#6366f1',
                      transition: 'width 0.5s ease',
                    }} />
                    <div style={{
                      width: `${(d.orderToAck / maxLatency) * 100}%`,
                      background: '#22d3ee',
                      transition: 'width 0.5s ease',
                    }} />
                    <div style={{
                      width: `${(d.ackToFill / maxLatency) * 100}%`,
                      background: '#10b981',
                      transition: 'width 0.5s ease',
                    }} />
                  </div>
                  <span style={{ width: 50, fontSize: '0.7rem', color: 'var(--text-secondary)', fontFamily: 'var(--font-mono)', textAlign: 'right' }}>
                    {total.toFixed(1)}ms
                  </span>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
