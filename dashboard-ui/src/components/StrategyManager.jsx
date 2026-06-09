import { useApi, apiPost } from '../hooks/useApi';

export default function StrategyManager() {
  const { data, refetch } = useApi('/strategies', 3000);
  const strategies = data?.strategies || [];

  const handleAction = async (strategyId, action) => {
    await apiPost(`/strategies/${strategyId}/${action}`);
    setTimeout(refetch, 500);
  };

  return (
    <div className="card strategy-card-wrapper">
      <div className="card-header">
        <span className="card-title">🎯 Strategies</span>
        <span style={{ fontFamily: 'var(--font-mono)', fontSize: '0.75rem', color: 'var(--text-muted)' }}>
          {strategies.length} registered
        </span>
      </div>
      <div>
        {strategies.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon">🎯</div>
            <div>No strategies registered</div>
            <div style={{ fontSize: '0.75rem', marginTop: 4 }}>Start the trading core to register strategies</div>
          </div>
        ) : (
          strategies.map((s) => (
            <div className="strategy-item" key={s.strategy_id}>
              <div className="strategy-info">
                <h3>{s.strategy_id}</h3>
                <div className="strategy-meta">
                  <span className={`state-badge ${s.state?.toLowerCase()}`}>
                    {s.state}
                  </span>
                </div>
              </div>
              <div className="strategy-controls">
                {s.state !== 'RUNNING' && (
                  <button className="btn-sm start" onClick={() => handleAction(s.strategy_id, 'start')}>
                    ▶ Start
                  </button>
                )}
                {s.state === 'RUNNING' && (
                  <button className="btn-sm pause" onClick={() => handleAction(s.strategy_id, 'pause')}>
                    ⏸ Pause
                  </button>
                )}
                {s.state !== 'STOPPED' && (
                  <button className="btn-sm stop" onClick={() => handleAction(s.strategy_id, 'stop')}>
                    ⏹ Stop
                  </button>
                )}
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
