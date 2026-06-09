import { useApi } from '../hooks/useApi';

export default function ExposureGauge() {
  const { data } = useApi('/exposure', 3000);
  const exposure = data?.exposure || {};
  const maxExposure = 100000; // From settings.max_exposure_per_strategy

  const entries = Object.entries(exposure);

  return (
    <div className="card exposure-card">
      <div className="card-header">
        <span className="card-title">💰 Exposure</span>
        <span style={{ fontFamily: 'var(--font-mono)', fontSize: '0.75rem', color: 'var(--text-muted)' }}>
          ₹{(data?.total || 0).toLocaleString('en-IN')} total
        </span>
      </div>
      <div className="card-body">
        {entries.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon">💰</div>
            <div>No active exposure</div>
          </div>
        ) : (
          entries.map(([strategy, amount]) => {
            const pct = Math.min((amount / maxExposure) * 100, 100);
            const isHigh = pct > 75;
            return (
              <div className="exposure-bar-wrapper" key={strategy}>
                <div className="exposure-bar-label">
                  <span className="exposure-bar-name">{strategy}</span>
                  <span className="exposure-bar-value">
                    ₹{Number(amount).toLocaleString('en-IN')} / ₹{maxExposure.toLocaleString('en-IN')}
                  </span>
                </div>
                <div className="exposure-bar-track">
                  <div
                    className={`exposure-bar-fill ${isHigh ? 'high' : ''}`}
                    style={{ width: `${pct}%` }}
                  />
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
