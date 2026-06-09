import { useApi } from '../hooks/useApi';

export default function StatsRow() {
  const { data: pnl } = useApi('/pnl', 1000);
  const { data: positions } = useApi('/positions', 2000);
  const { data: prices } = useApi('/prices', 1000);

  const net = pnl?.net || 0;
  const realized = pnl?.realized || 0;
  const posCount = positions?.positions?.length || 0;

  // Get a representative price
  const priceEntries = prices?.prices ? Object.entries(prices.prices) : [];

  return (
    <div className="stats-row">
      <div className="stat-card">
        <div className="stat-label">Net P&L</div>
        <div className={`stat-value ${net >= 0 ? 'profit' : 'loss'}`}>
          ₹{net >= 0 ? '+' : ''}{net.toLocaleString('en-IN', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
        </div>
        <div className="stat-sub">Realized: ₹{realized.toFixed(2)}</div>
      </div>

      <div className="stat-card">
        <div className="stat-label">Open Positions</div>
        <div className="stat-value">{posCount}</div>
        <div className="stat-sub">{posCount === 0 ? 'No active positions' : `${posCount} instrument${posCount > 1 ? 's' : ''}`}</div>
      </div>

      {priceEntries.slice(0, 2).map(([inst, price]) => (
        <div className="stat-card" key={inst}>
          <div className="stat-label">{inst.replace('NSE:', '')}</div>
          <div className="stat-value" style={{ color: 'var(--accent-cyan)' }}>
            ₹{Number(price).toLocaleString('en-IN', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
          </div>
          <div className="stat-sub">LTP</div>
        </div>
      ))}

      {priceEntries.length === 0 && (
        <>
          <div className="stat-card">
            <div className="stat-label">NIFTY</div>
            <div className="stat-value" style={{ color: 'var(--text-muted)' }}>--</div>
            <div className="stat-sub">Awaiting data</div>
          </div>
          <div className="stat-card">
            <div className="stat-label">BANKNIFTY</div>
            <div className="stat-value" style={{ color: 'var(--text-muted)' }}>--</div>
            <div className="stat-sub">Awaiting data</div>
          </div>
        </>
      )}
    </div>
  );
}
