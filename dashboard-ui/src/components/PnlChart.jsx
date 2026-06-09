import { useEffect, useRef, useState } from 'react';
import { createChart, AreaSeries } from 'lightweight-charts';
import { useApi } from '../hooks/useApi';

export default function PnlChart() {
  const chartRef = useRef(null);
  const chartInstance = useRef(null);
  const seriesRef = useRef(null);
  const { data: pnl } = useApi('/pnl', 1500);
  const [pnlHistory, setPnlHistory] = useState([]);

  useEffect(() => {
    if (!chartRef.current) return;

    const chart = createChart(chartRef.current, {
      width: chartRef.current.clientWidth,
      height: 300,
      layout: {
        background: { type: 'solid', color: 'transparent' },
        textColor: '#94a3b8',
        fontFamily: "'Inter', sans-serif",
        fontSize: 11,
      },
      grid: {
        vertLines: { color: 'rgba(148, 163, 184, 0.06)' },
        horzLines: { color: 'rgba(148, 163, 184, 0.06)' },
      },
      crosshair: {
        mode: 0,
        vertLine: { color: 'rgba(99, 102, 241, 0.4)', width: 1, style: 2 },
        horzLine: { color: 'rgba(99, 102, 241, 0.4)', width: 1, style: 2 },
      },
      rightPriceScale: {
        borderColor: 'rgba(148, 163, 184, 0.1)',
      },
      timeScale: {
        borderColor: 'rgba(148, 163, 184, 0.1)',
        timeVisible: true,
        secondsVisible: true,
      },
    });

    // lightweight-charts v5 API
    const series = chart.addSeries(AreaSeries, {
      lineColor: '#6366f1',
      topColor: 'rgba(99, 102, 241, 0.3)',
      bottomColor: 'rgba(99, 102, 241, 0.02)',
      lineWidth: 2,
      crosshairMarkerBackgroundColor: '#6366f1',
      priceFormat: { type: 'custom', formatter: (p) => '₹' + p.toFixed(2) },
    });

    chartInstance.current = chart;
    seriesRef.current = series;

    const handleResize = () => {
      if (chartRef.current) {
        chart.applyOptions({ width: chartRef.current.clientWidth });
      }
    };
    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      chart.remove();
    };
  }, []);

  useEffect(() => {
    if (!pnl || !seriesRef.current) return;
    const net = pnl.net || 0;
    const now = Math.floor(Date.now() / 1000);

    setPnlHistory(prev => {
      const updated = [...prev, { time: now, value: net }];
      const trimmed = updated.slice(-500);
      seriesRef.current.setData(trimmed);
      return trimmed;
    });
  }, [pnl]);

  return (
    <div className="card pnl-chart-card">
      <div className="card-header">
        <span className="card-title">📈 Equity Curve</span>
        <span style={{ fontFamily: 'var(--font-mono)', fontSize: '0.75rem', color: 'var(--text-muted)' }}>
          LIVE
        </span>
      </div>
      <div className="card-body">
        <div ref={chartRef} className="chart-container" />
      </div>
    </div>
  );
}
