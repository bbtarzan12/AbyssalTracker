import React, { useMemo } from 'react';
import {
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  ComposedChart,
  Line,
} from 'recharts';
import './OverallStatsDisplay.css';
import type { RunData, OverallStats } from "../types";

interface OverallStatsDisplayProps {
  df: RunData[];
  overall_stats: OverallStats;
}

const OverallStatsDisplay: React.FC<OverallStatsDisplayProps> = ({ df, overall_stats }) => {
  // 금액 포맷팅 함수
  const formatISK = (amount: number): string => {
    const absAmount = Math.abs(amount);
    const sign = amount < 0 ? '-' : '';
    
    if (absAmount >= 1000000000) { // 1billion 이상
      return `${sign}${(absAmount / 1000000000).toFixed(2)}b`;
    } else if (absAmount >= 1000000) { // 1million 이상
      return `${sign}${(absAmount / 1000000).toFixed(1)}m`;
    } else { // 1million 미만
      return `${sign}${Math.round(absAmount).toLocaleString()}`;
    }
  };

  if (!overall_stats) {
    return (
      <div className="no-data-container">
        <div className="no-data-icon">📊</div>
        <h3>📊 전체 통계 데이터 없음</h3>
        <p>전체 통계 데이터가 없습니다. 어비셜 런 데이터를 수집한 후 다시 확인해보세요.</p>
      </div>
    );
  }

  // Calculate derived statistics
  const totalRuns = df.length;
  const totalProfit = df.reduce((sum, run) => sum + run['실수익'], 0);
  const totalPlayTimeMinutes = df.reduce((sum, run) => sum + run['런 소요(분)'], 0);
  const totalPlayHours = Math.floor(totalPlayTimeMinutes / 60);
  const totalPlayRemainingMinutes = Math.round(totalPlayTimeMinutes % 60);

  // Daily trend data for chart
  const dailyTrendData = useMemo(() => {
    const dailyData = df.reduce((acc, run) => {
      const date = run['날짜'];
      if (!acc[date]) {
        acc[date] = { runs: [], totalProfit: 0 };
      }
      acc[date].runs.push(run);
      acc[date].totalProfit += run['실수익'];
      return acc;
    }, {} as Record<string, { runs: RunData[], totalProfit: number }>);

    return Object.entries(dailyData)
      .map(([date, data]) => ({
        date,
        totalProfit: data.totalProfit,
        avgProfit: data.totalProfit / data.runs.length,
        runCount: data.runs.length
      }))
      .sort((a, b) => a.date.localeCompare(b.date));
  }, [df]);

  return (
    <div className="overall-analytics">
      {/* Top Section: Key Metrics & Daily Trend Chart */}
      <div className="top-section">
        {/* Key Metrics Overview */}
        <div className="metric-card highlight combined-card">
          <div className="metric-header">
            <div className="metric-title">📊 전체 현황</div>
          </div>
          <div className="combined-metrics">
            <div className="combined-metric">
              <div className="combined-label">총 런 수</div>
              <div className="combined-value">{totalRuns.toLocaleString()}</div>
            </div>
            <div className="combined-metric">
              <div className="combined-label">총 수익</div>
              <div className="combined-value">{formatISK(totalProfit)}</div>
            </div>
            <div className="combined-metric">
              <div className="combined-label">전체 플레이시간</div>
              <div className="combined-value">
                {totalPlayHours > 0 ? `${totalPlayHours}h ` : ''}{totalPlayRemainingMinutes}m
              </div>
            </div>
          </div>
        </div>

        {/* Daily Profit Trend Chart */}
        <div className="chart-container">
          <div className="chart-header">
            <div className="chart-header-left"></div>
            <h3 className="chart-title">📈 일별 수익 트렌드</h3>
            <div className="chart-legend">
              <div className="legend-item">
                <div className="legend-color" style={{ backgroundColor: '#50fa7b' }}></div>
                <span className="legend-text">💰 총 수익</span>
              </div>
              <div className="legend-item">
                <div className="legend-color" style={{ backgroundColor: '#bd93f9' }}></div>
                <span className="legend-text">📈 평균 수익</span>
              </div>
            </div>
          </div>
          <div className="chart-content">
            <ResponsiveContainer width="100%" height="100%">
              <ComposedChart data={dailyTrendData} margin={{ top: 20, right: 30, left: 20, bottom: 20 }}>
                <defs>
                  <linearGradient id="totalProfitGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#50fa7b" stopOpacity={0.4}/>
                    <stop offset="95%" stopColor="#50fa7b" stopOpacity={0.1}/>
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="var(--border-primary)" />
                <XAxis 
                  dataKey="date" 
                  stroke="var(--text-muted)" 
                  tick={{ fill: 'var(--text-muted)', fontSize: 11 }}
                  tickFormatter={(value) => value.slice(5)} // MM-DD 형식으로 표시
                />
                <YAxis 
                  yAxisId="left"
                  stroke="#50fa7b" 
                  tick={{ fill: '#50fa7b', fontSize: 11 }}
                  tickFormatter={(value) => formatISK(value)}
                  label={{ value: '총 수익', angle: -90, position: 'insideLeft', style: { textAnchor: 'middle', fill: '#50fa7b' } }}
                />
                <YAxis 
                  yAxisId="right"
                  orientation="right"
                  stroke="#bd93f9" 
                  tick={{ fill: '#bd93f9', fontSize: 11 }}
                  tickFormatter={(value) => formatISK(value)}
                  label={{ value: '평균 수익', angle: 90, position: 'insideRight', style: { textAnchor: 'middle', fill: '#bd93f9' } }}
                />
                <Tooltip 
                  contentStyle={{
                    backgroundColor: 'var(--surface-bg)',
                    border: '1px solid var(--border-primary)',
                    borderRadius: 'var(--radius-md)',
                    color: 'var(--text-primary)'
                  }}
                  formatter={(value, _, props) => [
                    `${formatISK(value as number)}`,
                    props.dataKey === 'totalProfit' ? '💰 총 수익' : '📈 평균 수익'
                  ]}
                  labelFormatter={(label) => `📅 날짜: ${label}`}
                />
                <Area 
                  yAxisId="left"
                  type="monotone" 
                  dataKey="totalProfit" 
                  stroke="#50fa7b" 
                  fill="url(#totalProfitGradient)"
                  strokeWidth={2}
                  name="💰 총 수익"
                />
                <Line 
                  yAxisId="right"
                  type="monotone" 
                  dataKey="avgProfit" 
                  stroke="#bd93f9" 
                  strokeWidth={3}
                  dot={{ fill: '#bd93f9', strokeWidth: 2, r: 5 }}
                  activeDot={{ r: 7, stroke: '#bd93f9', strokeWidth: 2, fill: '#ffffff' }}
                  name="📈 평균 수익"
                />
              </ComposedChart>
            </ResponsiveContainer>
          </div>
        </div>
      </div>

      {/* Abyssal Weather Statistics Table */}
      <div className="weather-stats-container">
        <div className="metric-header">
          <div className="metric-title">🌪️ 어비셜 날씨별 통계</div>
        </div>
        
        {overall_stats.tier_weather_stats && overall_stats.tier_weather_stats.length > 0 ? (
          <div className="weather-stats-table-container">
            <table className="weather-stats-table">
              <thead>
                <tr>
                  <th>티어</th>
                  <th>날씨</th>
                  <th>런 수</th>
                  <th>총 플레이 시간</th>
                  <th>총 수익</th>
                  <th>총 입장료</th>
                  <th>평균 수익</th>
                  <th>평균 시간당 수익</th>
                  <th>평균 진행 시간</th>
                  <th>수익성</th>
                </tr>
              </thead>
              <tbody>
                {overall_stats.tier_weather_stats
                  .sort((a, b) => {
                    // 티어별로 먼저 정렬 (T1, T2, T3, T4, T5 순)
                    const tierOrder = ['T1', 'T2', 'T3', 'T4', 'T5'];
                    const aTierIndex = tierOrder.indexOf(a.tier);
                    const bTierIndex = tierOrder.indexOf(b.tier);
                    if (aTierIndex !== bTierIndex) {
                      return aTierIndex - bTierIndex;
                    }
                    // 같은 티어 내에서는 날씨별로 정렬
                    return a.weather.localeCompare(b.weather);
                  })
                  .map((stat, index) => {
                    const totalProfit = stat.avg_isk * stat.runs_count;
                    const totalPlayTime = stat.avg_time * stat.runs_count;
                    const totalPlayHours = Math.floor(totalPlayTime / 60);
                    const totalPlayMinutes = Math.round(totalPlayTime % 60);
                    
                    // 수익성 퍼센트 계산
                    const profitabilityPercent = stat.total_entry_cost > 0 ? (totalProfit / stat.total_entry_cost * 100) : 0;
                    
                    return (
                      <tr key={index} className="weather-stats-row">
                        <td className="tier-cell">
                          <span className={`tier-badge tier-${stat.tier.toLowerCase()}`}>
                            {stat.tier}
                          </span>
                        </td>
                        <td className="weather-cell">
                          <span className="weather-name">{stat.weather}</span>
                        </td>
                        <td className="runs-cell">
                          <span className="runs-count">{stat.runs_count.toLocaleString()}</span>
                        </td>
                        <td className="time-cell">
                          <span className="time-value">
                            {totalPlayHours > 0 ? `${totalPlayHours}h ` : ''}{totalPlayMinutes}m
                          </span>
                        </td>
                        <td className="total-profit-cell">
                          <span className={`profit-value ${totalProfit >= 0 ? 'positive' : 'negative'}`}>
                            {formatISK(totalProfit)}
                          </span>
                        </td>
                        <td className="entry-cost-cell">
                          <span className="entry-cost-value">{formatISK(stat.total_entry_cost)}</span>
                        </td>
                        <td className="avg-profit-cell">
                          <span className={`profit-value ${stat.avg_isk >= 0 ? 'positive' : 'negative'}`}>
                            {formatISK(stat.avg_isk)}
                          </span>
                        </td>
                        <td className="iskph-cell">
                          <span className="iskph-value">{formatISK(stat.avg_iskph)}/h</span>
                        </td>
                        <td className="time-cell">
                          <span className="time-value">{stat.avg_time.toFixed(1)}분</span>
                        </td>
                        <td className="profitability-cell">
                          <span className={`profitability-value ${profitabilityPercent >= 100 ? 'positive' : profitabilityPercent >= 50 ? 'neutral' : 'negative'}`}>
                            {profitabilityPercent.toFixed(1)}%
                          </span>
                        </td>
                      </tr>
                    );
                  })}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="no-weather-data">
            <p>어비셜 날씨별 통계 데이터가 없습니다.</p>
          </div>
        )}
      </div>
    </div>
  );
};

export default OverallStatsDisplay;