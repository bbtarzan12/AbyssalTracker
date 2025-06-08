import React from 'react';
import { XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, AreaChart, Area } from 'recharts';
import { parseItems, aggregateItems } from './utils';
import './DailyStatsDisplay.css';
import type { RunData, DailyStats } from "../types";

interface DailySummaryProps {
  df: RunData[];
  daily_stats: DailyStats;
  item_buy_price_cache: { [key: string]: number };
  selectedDate: string;
  setSelectedDate: (date: string) => void;
}

const DailySummary: React.FC<DailySummaryProps> = ({
  df,
  daily_stats,
  item_buy_price_cache,
  selectedDate,
  setSelectedDate,
}) => {
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

  if (!daily_stats || Object.keys(daily_stats).length === 0) {
    return (
      <div className="no-data-container">
        <div className="no-data-icon">📊</div>
        <h3>📅 일별 데이터 없음</h3>
        <p>일별 통계 데이터가 없습니다. 어비셜 런을 기록한 후 다시 확인해보세요.</p>
      </div>
    );
  }

  const dates = Object.keys(daily_stats).sort().reverse();
  const currentDailyData = daily_stats[selectedDate];
  
  const totalDailyIsk = currentDailyData ? currentDailyData.runs.reduce((sum, run) => sum + run['실수익'], 0) : 0;
  const totalRuns = currentDailyData ? currentDailyData.runs.length : 0;
  const avgProfit = totalRuns > 0 ? totalDailyIsk / totalRuns : 0;

  const filteredDfDaily = df.filter(run => run['날짜'] === selectedDate);

  // 오늘의 기록 계산
  const todayRuns = currentDailyData ? currentDailyData.runs : [];
  
  // 베스트 & 워스트 런
  const bestProfitRun = todayRuns.length > 0 ? todayRuns.reduce((best, run) => 
    run['실수익'] > best['실수익'] ? run : best
  ) : null;
  
  const worstProfitRun = todayRuns.length > 0 ? todayRuns.reduce((worst, run) => 
    run['실수익'] < worst['실수익'] ? run : worst
  ) : null;
  
  const fastestRun = todayRuns.length > 0 ? todayRuns.reduce((fastest, run) => 
    (run['런 소요(분)'] || 999) < (fastest['런 소요(분)'] || 999) ? run : fastest
  ) : null;
  
  const slowestRun = todayRuns.length > 0 ? todayRuns.reduce((slowest, run) => 
    (run['런 소요(분)'] || 0) > (slowest['런 소요(분)'] || 0) ? run : slowest
  ) : null;

  // 가장 비싼 아이템 찾기
  let mostExpensiveItem = { name: '', value: 0 };
  
  todayRuns.forEach(run => {
    if (run['획득 아이템']) {
      const parsedItems = parseItems(run['획득 아이템']);
      const aggregatedItems = aggregateItems(parsedItems, item_buy_price_cache);
      
      aggregatedItems.forEach((item: any) => {
        if (item['총 가격'] > mostExpensiveItem.value) {
          mostExpensiveItem = {
            name: item['아이템 이름'],
            value: item['총 가격']
          };
        }
      });
    }
  });

  // 기본 정보 계산
  const totalPlayTime = todayRuns.reduce((total, run) => total + (run['런 소요(분)'] || 0), 0);
  const totalPlayHours = Math.floor(totalPlayTime / 60);
  const totalPlayMinutes = Math.round(totalPlayTime % 60);
  


  return (
    <div className="daily-analytics">
      {/* Filter Controls */}
      <div className="filter-controls">
        <div className="filter-group">
          <label className="filter-label">📅 날짜 선택</label>
          <select
            value={selectedDate}
            onChange={(e) => setSelectedDate(e.target.value)}
            className="filter-select"
            key={`date-select-${dates.length}`}
          >
            {dates.map(date => (
              <option key={date} value={date}>{date}</option>
            ))}
          </select>
          <div className="filter-status">
            <span className="status-badge success">🚀 {totalRuns}번 런</span>
          </div>
        </div>
        
        {/* Compact Metrics in Filter Controls */}
        <div className="filter-metrics">
          <div className="metric-compact">
            <span className="metric-label-compact">💰 총 수익</span>
            <span className="metric-value-compact primary">{formatISK(totalDailyIsk)}</span>
          </div>
          <div className="metric-compact">
            <span className="metric-label-compact">📈 평균 수익</span>
            <span className="metric-value-compact">{formatISK(avgProfit)}</span>
          </div>
          <div className="metric-compact">
            <span className="metric-label-compact">⏱️ ISK/시간</span>
            <span className="metric-value-compact accent">
              {currentDailyData ? formatISK(currentDailyData.avg_iskph) : '0'}/h
            </span>
          </div>
          <div className="metric-compact">
            <span className="metric-label-compact">⏰ 평균 소요시간</span>
            <span className="metric-value-compact">
              {currentDailyData ? currentDailyData.avg_time.toFixed(1) : 0}min
            </span>
          </div>
        </div>
      </div>

      {/* Chart and Info Section */}
      <div className="chart-info-container">
        {/* Basic Info - Left Side */}
        {todayRuns.length > 0 && (
          <div className="basic-info-section">
            <h3 className="section-title">
              <span className="section-icon">📊</span>
              기본 정보
            </h3>
            <div className="info-cards">
              <div className="info-card playtime">
                <div className="info-label">⏰ 총 플레이 시간</div>
                <div className="info-value">
                  {totalPlayHours > 0 ? `${totalPlayHours}시간 ` : ''}{totalPlayMinutes}분
                </div>
                <div className="info-detail">({todayRuns.length}런 완료)</div>
              </div>

              {mostExpensiveItem.name && (
                <div className="info-card expensive">
                  <div className="info-label">📦 가장 비싼 아이템</div>
                  <div className="info-value">{mostExpensiveItem.name}</div>
                  <div className="info-detail">({formatISK(mostExpensiveItem.value)})</div>
                </div>
              )}
            </div>
          </div>
        )}

        {/* Performance Chart - Right Side */}
        <div className="chart-container">
          <div className="chart-header">
            <h3 className="chart-title">📈 일별 성과 트렌드</h3>
          </div>
          <ResponsiveContainer width="100%" height={225}>
            <AreaChart data={filteredDfDaily} margin={{ top: 20, right: 30, left: 20, bottom: 20 }}>
              <defs>
                <linearGradient id="profitGradient" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="var(--accent-bg)" stopOpacity={0.3}/>
                  <stop offset="95%" stopColor="var(--accent-bg)" stopOpacity={0}/>
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="var(--border-primary)" />
              <XAxis 
                dataKey="시작시각(KST)" 
                stroke="var(--text-muted)" 
                tick={{ fill: 'var(--text-muted)', fontSize: 12 }}
                tickFormatter={(value) => value.split(' ')[1]?.substring(0, 5) || ''}
              />
              <YAxis 
                stroke="var(--text-muted)" 
                tick={{ fill: 'var(--text-muted)', fontSize: 12 }}
                tickFormatter={(value) => formatISK(value)}
              />
              <Tooltip 
                contentStyle={{
                  backgroundColor: 'var(--surface-bg)',
                  border: '1px solid var(--border-primary)',
                  borderRadius: 'var(--radius-md)',
                  color: 'var(--text-primary)'
                }}
                formatter={(value: number) => [`${formatISK(value)}`, '수익']}
                labelFormatter={(label) => `시간: ${label.split(' ')[1]}`}
              />
              <Area 
                type="monotone" 
                dataKey="실수익" 
                stroke="var(--accent-bg)" 
                fillOpacity={1} 
                fill="url(#profitGradient)"
                strokeWidth={2}
              />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      </div>

      {/* 오늘의 기록 섹션 */}
      {todayRuns.length > 0 && (
        <div className="daily-records-container">
          {/* 베스트 & 워스트 */}
          <div className="records-section">
            <h3 className="section-title">
              <span className="section-icon">🏆</span>
              베스트 & 워스트
            </h3>
            <div className="records-grid">
              {bestProfitRun && (
                <div className="record-card best">
                  <div className="record-label">🏆 최고 수익 런</div>
                  <div className="record-value">{formatISK(bestProfitRun['실수익'])}</div>
                  <div className="record-detail">({(bestProfitRun['런 소요(분)'] || 0).toFixed(1)}분)</div>
                </div>
              )}
              {worstProfitRun && (
                <div className="record-card worst">
                  <div className="record-label">💸 최저 수익 런</div>
                  <div className="record-value">{formatISK(worstProfitRun['실수익'])}</div>
                  <div className="record-detail">({(worstProfitRun['런 소요(분)'] || 0).toFixed(1)}분)</div>
                </div>
              )}
              {fastestRun && (
                <div className="record-card fast">
                  <div className="record-label">⚡ 최단시간 런</div>
                  <div className="record-value">{(fastestRun['런 소요(분)'] || 0).toFixed(1)}분</div>
                  <div className="record-detail">({formatISK(fastestRun['실수익'])})</div>
                </div>
              )}
              {slowestRun && (
                <div className="record-card slow">
                  <div className="record-label">🐌 최장시간 런</div>
                  <div className="record-value">{(slowestRun['런 소요(분)'] || 0).toFixed(1)}분</div>
                  <div className="record-detail">({formatISK(slowestRun['실수익'])})</div>
                </div>
              )}
            </div>
          </div>


        </div>
      )}
    </div>
  );
};

export default DailySummary; 