import React, { useMemo } from 'react';
import { ComposedChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer, Area } from 'recharts';
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
  const bestRun = df.reduce((best, run) => run['실수익'] > best['실수익'] ? run : best, df[0] || {});
  // const worstRun = df.reduce((worst, run) => run['실수익'] < worst['실수익'] ? run : worst, df[0] || {});



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
      {/* Key Metrics Overview */}
      <div className="analytics-grid">
        <div className="metric-card highlight">
          <div className="metric-header">
            <div className="metric-title">🚀 총 런 수</div>
            <div className="metric-icon">🚀</div>
          </div>
          <div className="metric-value">{totalRuns.toLocaleString()}</div>
          <div className="metric-change positive">
            <span>📈</span>
            전체 데이터셋
          </div>
        </div>

        <div className="metric-card highlight">
          <div className="metric-header">
            <div className="metric-title">💰 총 수익</div>
            <div className="metric-icon">💰</div>
          </div>
          <div className="metric-value">{formatISK(totalProfit)}</div>
          <div className="metric-change positive">
            <span>💎</span>
            누적 수익
          </div>
        </div>

        <div className="metric-card highlight">
          <div className="metric-header">
            <div className="metric-title">⚡ 평균 시간 당 수익</div>
            <div className="metric-icon">⚡</div>
          </div>
          <div className="metric-value">{formatISK(overall_stats.avg_iskph)}/h</div>
          <div className="metric-change neutral">
            <span>⚖️</span>
            평균 {(overall_stats.avg_time).toFixed(1)}분
          </div>
        </div>

        <div className="metric-card highlight">
          <div className="metric-header">
            <div className="metric-title">🏆 최고 성과</div>
            <div className="metric-icon">🏆</div>
          </div>
          <div className="metric-value">{formatISK(bestRun['실수익'])}</div>
          <div className="metric-change positive">
            <span>🎯</span>
            단일 런 기록
          </div>
        </div>
      </div>

      {/* Daily Profit Trend Chart */}
      <div className="chart-container">
        <div className="chart-header">
          <h3 className="chart-title">📈 일별 수익 트렌드</h3>
          <p className="chart-subtitle">날짜별 총 수익 및 평균 수익 변화</p>
        </div>
        <ResponsiveContainer width="100%" height={350}>
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
            <Legend 
              wrapperStyle={{ paddingTop: '20px' }}
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
  );
};

export default OverallStatsDisplay;