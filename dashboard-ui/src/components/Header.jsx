import { apiPost } from '../hooks/useApi';

export default function Header({ connected, killSwitchActive, onKillSwitch }) {
  return (
    <header className="header">
      <div className="header-left">
        <div className="header-logo">
          <h1>⚡ MayaviTrader</h1>
          <span className="version">v0.1.0</span>
        </div>
        <div className="connection-status">
          <span className={`status-dot ${connected ? '' : 'disconnected'}`}></span>
          {connected ? 'CONNECTED' : 'DISCONNECTED'}
        </div>
      </div>
      <div className="header-right">
        <span className="mode-badge paper">PAPER MODE</span>
        <button
          className="kill-switch-btn"
          onClick={async () => {
            if (window.confirm('⚠️ Activate KILL SWITCH? This will cancel all orders.')) {
              await apiPost('/kill-switch');
              onKillSwitch?.();
            }
          }}
        >
          🔴 KILL SWITCH
        </button>
      </div>
    </header>
  );
}
