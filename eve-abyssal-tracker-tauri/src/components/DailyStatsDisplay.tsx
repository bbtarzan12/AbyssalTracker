import React, { useState } from 'react';
import { XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, AreaChart, Area } from 'recharts';
import { parseItems, aggregateItems, ItemIcon, RunTypeBadge } from './utils';
import './DailyStatsDisplay.css';

interface RunData {
  '시작시각(KST)': string;
  '종료시각(KST)': string;
  '런 소요(분)': number;
  '어비셜 종류': string;
  '실수익': number;
  'ISK/h': number;
  '획득 아이템': string;
  '날짜': string;
}

interface DailyStats {
  [date: string]: {
    runs: RunData[];
    avg_isk: number;
    avg_time: number;
    avg_iskph: number;
  };
}

interface DailyStatsDisplayProps {
  df: RunData[];
  daily_stats: DailyStats;
  item_buy_price_cache: { [key: string]: number };
  selectedDate: string;
  setSelectedDate: (date: string) => void;
}

const DailyStatsDisplay: React.FC<DailyStatsDisplayProps> = ({
  df,
  daily_stats,
  item_buy_price_cache,
  selectedDate,
  setSelectedDate,
}) => {
  console.log("[DEBUG DailyStatsDisplay] daily_stats:", daily_stats);
  console.log("[DEBUG DailyStatsDisplay] selectedDate:", selectedDate);

  // 금액 포맷팅 함수
  const formatISK = (amount: number): string => {
    if (amount >= 1000000000) { // 1billion 이상
      return `${(amount / 1000000000).toFixed(2)}b`;
    } else if (amount >= 1000000) { // 1million 이상
      return `${(amount / 1000000).toFixed(1)}m`;
    } else { // 1million 미만
      return `${Math.round(amount).toLocaleString()}`;
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
  console.log("[DEBUG DailyStatsDisplay] dates:", dates);
  console.log("[DEBUG DailyStatsDisplay] currentDailyData:", currentDailyData);
  
  const totalDailyIsk = currentDailyData ? currentDailyData.runs.reduce((sum, run) => sum + run['실수익'], 0) : 0;
  const totalRuns = currentDailyData ? currentDailyData.runs.length : 0;
  const avgProfit = totalRuns > 0 ? totalDailyIsk / totalRuns : 0;

  const filteredDfDaily = df.filter(run => run['날짜'] === selectedDate);
  
  // Duration calculations
  const durations = filteredDfDaily.map(run => run['런 소요(분)']);
  const minDuration = durations.length > 0 ? Math.min(...durations) : 0;
  const maxDuration = durations.length > 0 ? Math.max(...durations) : 0;

  // 각 런의 펼침/접힘 상태를 관리하는 상태
  const [expandedRuns, setExpandedRuns] = useState<Record<number, boolean>>({});

  // 런 제목 클릭 시 상태 토글 함수
  const toggleRunExpansion = (index: number) => {
    setExpandedRuns(prev => ({
      ...prev,
      [index]: !prev[index]
    }));
  };

  // 성능 트렌드 계산
  const performanceTrend = filteredDfDaily.length > 1 ? 
    ((filteredDfDaily[filteredDfDaily.length - 1]['실수익'] - filteredDfDaily[0]['실수익']) / filteredDfDaily[0]['실수익'] * 100) : 0;

  return (
    <div className="daily-analytics">
      {/* Filter Controls */}
      <div className="filter-controls">
        <div className="filter-group">
          <label className="filter-label">📅 날짜 선택</label>
          <div className="filter-row">
            <select
              value={selectedDate}
              onChange={(e) => setSelectedDate(e.target.value)}
              className="filter-select"
            >
              {dates.map(date => (
                <option key={date} value={date}>{date}</option>
              ))}
            </select>
            <div className="filter-status">
              <span className="status-badge success">🚀 {totalRuns}번 런</span>
              <span className="status-badge info">{selectedDate}</span>
            </div>
          </div>
        </div>
      </div>

      {/* Compact Analytics Grid */}
      <div className="compact-analytics-grid">
        {/* Unified Metrics Card */}
        <div className="metric-card unified">
          <div className="metric-header">
            <div className="metric-title">📊 일별 분석</div>
            <div className="metric-icon">📊</div>
          </div>
          <div className="metrics-grid">
            {/* Row 1: Profit Metrics */}
            <div className="metric-item highlight">
              <div className="metric-label">💰 총 수익</div>
              <div className="metric-right">
                <div className="metric-value-compact primary">{formatISK(totalDailyIsk)}</div>
                <div className={`metric-change ${performanceTrend >= 0 ? 'positive' : 'negative'}`}>
                  <span>{performanceTrend >= 0 ? '▲' : '▼'}</span>
                  {Math.abs(performanceTrend).toFixed(1)}%
                </div>
              </div>
            </div>
            <div className="metric-item">
              <div className="metric-label">📈 평균 수익</div>
              <div className="metric-right">
                <div className="metric-value-compact">{formatISK(avgProfit)}</div>
                <div className="metric-change neutral">
                  <span>⚊</span>
                                      런당
                </div>
              </div>
            </div>
            <div className="metric-item">
              <div className="metric-label">⏱️ ISK/시간</div>
              <div className="metric-right">
                <div className="metric-value-compact accent">
                  {currentDailyData ? formatISK(currentDailyData.avg_iskph) : '0'}/h
                </div>
                <div className="metric-change positive">
                  <span>📈</span>
                                      효율성
                </div>
              </div>
            </div>
            <div className="metric-item">
              <div className="metric-label">⏰ 평균 소요시간</div>
              <div className="metric-right">
                <div className="metric-value-compact">
                  {currentDailyData ? currentDailyData.avg_time.toFixed(1) : 0}min
                </div>
                <div className="metric-change neutral">
                  <span>⏰</span>
                                      평균 시간
                </div>
              </div>
            </div>
            <div className="metric-item">
              <div className="metric-label">⚡ 최단 시간</div>
              <div className="metric-right">
                <div className="metric-value-compact success">
                  {minDuration.toFixed(1)}min
                </div>
                <div className="metric-change positive">
                  <span>⚡</span>
                                      최고 속도
                </div>
              </div>
            </div>
            <div className="metric-item">
              <div className="metric-label">🐌 최장 시간</div>
              <div className="metric-right">
                <div className="metric-value-compact warning">
                  {maxDuration.toFixed(1)}min
                </div>
                <div className="metric-change negative">
                  <span>🐌</span>
                                      가장 느림
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Performance Chart */}
        <div className="chart-container compact">
          <div className="chart-header">
            <h3 className="chart-title">📈 일별 성과 트렌드</h3>
            <p className="chart-subtitle">{selectedDate} 시간대별 ISK 수익 추이</p>
          </div>
          <ResponsiveContainer width="100%" height={280}>
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

      {/* Runs Table */}
      <div className="data-table-container">
        <div className="data-table-header">
          <h3 className="table-title">🚀 런 상세 정보</h3>
          <div className="table-actions">
            <button className="toolbar-btn" onClick={() => setExpandedRuns({})}>
              <span>📋</span>
              모두 접기
            </button>
          </div>
        </div>
        
        <div className="runs-table">
          {currentDailyData && currentDailyData.runs.map((run, i) => {
            const parsedItems = run['획득 아이템'] ? parseItems(run['획득 아이템']) : [];
            const aggregatedItems = aggregateItems(parsedItems, item_buy_price_cache);
            const isExpanded = expandedRuns[i];
            
            return (
              <div key={i} className={`run-card ${isExpanded ? 'expanded' : ''}`}>
                <div className="run-header" onClick={() => toggleRunExpansion(i)}>
                  <div className="run-meta">
                    <div className="run-time-badge">
                      {run['시작시각(KST)'].split(' ')[1]?.substring(0, 5)}
                    </div>
                    <RunTypeBadge abyssalType={run['어비셜 종류']} />
                  </div>
                  <div className="run-metrics">
                    <div className="run-metric">
                      <span className="metric-label">💰 수익</span>
                      <span className="metric-value-sm">{formatISK(run['실수익'])}</span>
                    </div>
                    <div className="run-metric">
                      <span className="metric-label">⏱️ ISK/h</span>
                      <span className="metric-value-sm">{formatISK(run['ISK/h'])}/h</span>
                    </div>
                    <div className="run-metric duration-metric">
                      <span className="metric-label">⏰ 소요시간</span>
                      <div className="duration-gauge-container">
                        <div className="duration-gauge">
                          <div 
                            className="duration-gauge-fill"
                            style={{
                              width: `${Math.min((run['런 소요(분)'] || 0) / 20 * 100, 100)}%`,
                              backgroundColor: (run['런 소요(분)'] || 0) <= 12 ? 'var(--success)' : 
                                             (run['런 소요(분)'] || 0) <= 15 ? 'var(--warning)' : 'var(--error)'
                            }}
                          ></div>
                          <span className="duration-text">{run['런 소요(분)']?.toFixed(1)}min</span>
                        </div>
                      </div>
                    </div>
                  </div>
                  <div className="run-expand">
                    <span className={`expand-icon ${isExpanded ? 'expanded' : ''}`}>▶</span>
                  </div>
                </div>
                
                {isExpanded && (
                  <div className="run-details">
                    <div className="run-timeline">
                      <div className="timeline-item">
                        <div className="timeline-icon">🚀</div>
                        <div className="timeline-content">
                          <div className="timeline-title">시작</div>
                          <div className="timeline-time">{run['시작시각(KST)']}</div>
                        </div>
                      </div>
                      <div className="timeline-item">
                        <div className="timeline-icon">🏁</div>
                        <div className="timeline-content">
                          <div className="timeline-title">완료</div>
                          <div className="timeline-time">{run['종료시각(KST)']}</div>
                        </div>
                      </div>
                    </div>
                    
                    {aggregatedItems.length > 0 && (
                      <div className="loot-section">
                        <h4 className="section-title">
                          <span className="section-icon">🎁</span>
                          전리품 요약
                        </h4>
                        <div className="loot-grid">
                            {aggregatedItems.map((item: any, idx: number) => (
                            <div key={idx} className="loot-item">
                              <div className="loot-item-content">
                                <ItemIcon 
                                  itemName={item['아이템 이름']} 
                                  size={24}
                                  className="loot-item-icon"
                                />
                                <div className="loot-item-name">{item['아이템 이름']}</div>
                                <div className="loot-item-stats">
                                  <span className="quantity">×{item['개수']}</span>
                                  <span className="divider">|</span>
                                  <span className="unit-price">{formatISK(item['개당 가격'])}</span>
                                  <span className="divider">|</span>
                                  <span className="total-price">{formatISK(item['총 가격'])}</span>
                                </div>
                              </div>
                            </div>
                          ))}
                        </div>
                      </div>
                      )}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
};

export default DailyStatsDisplay;