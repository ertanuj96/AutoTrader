import { useWebSocket } from './hooks/useWebSocket';
import Header from './components/Header';
import StatsRow from './components/StatsRow';
import PnlChart from './components/PnlChart';
import StrategyManager from './components/StrategyManager';
import PositionsPanel from './components/PositionsPanel';
import OrderBook from './components/OrderBook';
import LatencyMonitor from './components/LatencyMonitor';
import ExposureGauge from './components/ExposureGauge';

function App() {
  const { connected } = useWebSocket();

  return (
    <div className="app">
      <Header connected={connected} />
      <div className="dashboard-grid">
        <StatsRow />
        <PnlChart />
        <StrategyManager />
        <PositionsPanel />
        <OrderBook />
        <LatencyMonitor />
        <ExposureGauge />
      </div>
    </div>
  );
}

export default App;
