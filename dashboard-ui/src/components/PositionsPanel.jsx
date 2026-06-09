import { useApi } from '../hooks/useApi';

export default function PositionsPanel() {
  const { data } = useApi('/positions', 1500);
  const positions = data?.positions || [];

  return (
    <div className="card positions-card">
      <div className="card-header">
        <span className="card-title">📊 Positions</span>
        <span style={{ fontFamily: 'var(--font-mono)', fontSize: '0.75rem', color: 'var(--text-muted)' }}>
          {positions.length} open
        </span>
      </div>
      <div className="card-body">
        {positions.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon">📊</div>
            <div>No open positions</div>
          </div>
        ) : (
          <div className="table-wrapper">
            <table>
              <thead>
                <tr>
                  <th>Instrument</th>
                  <th>Side</th>
                  <th>Qty</th>
                  <th>Entry</th>
                  <th>LTP</th>
                  <th>P&L</th>
                </tr>
              </thead>
              <tbody>
                {positions.map((pos, i) => {
                  const pnl = pos.unrealized_pnl || 0;
                  return (
                    <tr key={i}>
                      <td style={{ color: 'var(--text-primary)', fontWeight: 600 }}>
                        {pos.instrument || pos.key}
                      </td>
                      <td>
                        <span className={`order-side ${pos.side?.toLowerCase()}`}>
                          {pos.side}
                        </span>
                      </td>
                      <td>{pos.quantity}</td>
                      <td>₹{Number(pos.entry_price || 0).toFixed(2)}</td>
                      <td>₹{Number(pos.ltp || pos.entry_price || 0).toFixed(2)}</td>
                      <td className={pnl >= 0 ? 'profit' : 'loss'}>
                        ₹{pnl >= 0 ? '+' : ''}{pnl.toFixed(2)}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
