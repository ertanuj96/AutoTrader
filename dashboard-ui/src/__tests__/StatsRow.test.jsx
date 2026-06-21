import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import StatsRow from '../components/StatsRow';

// Mock useApi to control what data the component sees
vi.mock('../hooks/useApi', () => ({
  useApi: vi.fn(),
}));

import { useApi } from '../hooks/useApi';

const mockUseApi = (pnlData, positionsData, pricesData) => {
  useApi.mockImplementation((endpoint) => {
    if (endpoint === '/pnl') return { data: pnlData };
    if (endpoint === '/positions') return { data: positionsData };
    if (endpoint === '/prices') return { data: pricesData };
    return { data: null };
  });
};

describe('StatsRow', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders Net P&L label', () => {
    mockUseApi(null, null, null);
    render(<StatsRow />);
    expect(screen.getByText('Net P&L')).toBeInTheDocument();
  });

  it('renders Open Positions label', () => {
    mockUseApi(null, null, null);
    render(<StatsRow />);
    expect(screen.getByText('Open Positions')).toBeInTheDocument();
  });

  it('shows zero positions when no data', () => {
    mockUseApi(null, null, null);
    render(<StatsRow />);
    expect(screen.getByText('No active positions')).toBeInTheDocument();
  });

  it('shows profit P&L with profit CSS class', () => {
    mockUseApi({ net: 25000, realized: 15000 }, null, null);
    const { container } = render(<StatsRow />);
    const pnlValue = container.querySelector('.stat-value.profit');
    expect(pnlValue).toBeInTheDocument();
    expect(pnlValue.textContent).toContain('+');
  });

  it('shows loss P&L with loss CSS class', () => {
    mockUseApi({ net: -5000, realized: -5000 }, null, null);
    const { container } = render(<StatsRow />);
    const pnlValue = container.querySelector('.stat-value.loss');
    expect(pnlValue).toBeInTheDocument();
  });

  it('shows correct position count', () => {
    const positions = [
      { symbol: 'NIFTY', qty: 50 },
      { symbol: 'BANKNIFTY', qty: 25 },
    ];
    mockUseApi(null, { positions }, null);
    render(<StatsRow />);
    expect(screen.getByText('2')).toBeInTheDocument();
    expect(screen.getByText('2 instruments')).toBeInTheDocument();
  });

  it('shows singular "instrument" for one position', () => {
    mockUseApi(null, { positions: [{ symbol: 'NIFTY' }] }, null);
    render(<StatsRow />);
    expect(screen.getByText('1 instrument')).toBeInTheDocument();
  });

  it('shows awaiting data placeholders when no prices', () => {
    mockUseApi(null, null, { prices: {} });
    render(<StatsRow />);
    const waitingElements = screen.getAllByText('Awaiting data');
    expect(waitingElements.length).toBeGreaterThan(0);
  });

  it('shows live prices when prices data available', () => {
    mockUseApi(null, null, { prices: { 'NSE:NIFTY': 18432.5 } });
    render(<StatsRow />);
    expect(screen.getByText('NIFTY')).toBeInTheDocument();
    expect(screen.getByText('LTP')).toBeInTheDocument();
  });

  it('formats P&L with Indian locale (₹)', () => {
    mockUseApi({ net: 100000, realized: 50000 }, null, null);
    render(<StatsRow />);
    // Should contain ₹ symbol
    const pnlCards = screen.getAllByText(/₹/);
    expect(pnlCards.length).toBeGreaterThan(0);
  });

  it('shows zero net P&L as profit (green)', () => {
    mockUseApi({ net: 0, realized: 0 }, null, null);
    const { container } = render(<StatsRow />);
    const pnlValue = container.querySelector('.stat-value.profit');
    expect(pnlValue).toBeInTheDocument();
  });
});
