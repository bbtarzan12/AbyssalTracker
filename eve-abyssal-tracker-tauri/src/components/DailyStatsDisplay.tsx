import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, AreaChart, Area } from 'recharts';
import { parseItems, aggregateItems, ItemIcon, RunTypeBadge } from './utils';
import ShipClassIcon from './ShipClassIcon';
import './DailyStatsDisplay.css';

interface RunData {
  '시작시각(KST)': string;
  '종료시각(KST)': string;
  '런 소요(분)': number;
  '어비셜 종류': string;
  '함급': number;
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
  onDataUpdate?: () => void; // 전체 데이터 업데이트 콜백
  onLightRefresh?: () => void; // 가벼운 데이터 새로고침 콜백
  onRunDeleted?: (run: RunData) => void; // 런 삭제 콜백 (UI 업데이트만)
}

const DailyStatsDisplay: React.FC<DailyStatsDisplayProps> = ({
  df,
  daily_stats,
  item_buy_price_cache,
  selectedDate,
  setSelectedDate,
  onRunDeleted,
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

  // 함급 표시 함수
  const getShipClassDisplay = (shipClass: number): string => {
    switch (shipClass) {
      case 3: return '프리깃 (3x)';
      case 2: return '디스트로이어 (2x)';
      case 1: return '크루저 (1x)';
      default: return `등급 ${shipClass}`;
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

  // 각 런의 펼침/접힘 상태를 관리하는 상태
  const [expandedRuns, setExpandedRuns] = useState<Record<number, boolean>>({});

  // 런 제목 클릭 시 상태 토글 함수
  const toggleRunExpansion = (index: number) => {
    setExpandedRuns(prev => ({
      ...prev,
      [index]: !prev[index]
    }));
  };

  const handleDeleteRun = async (run: RunData) => {
    try {
      // CSV에서만 삭제 (API 호출 없이) - 확인창 없이 바로 삭제
      await invoke('delete_abyssal_run_command', {
        startTimeKst: run['시작시각(KST)'],
        endTimeKst: run['종료시각(KST)']
      });
      
      console.log('[INFO] Run deleted from CSV successfully');
      
      // UI 상태에서만 해당 런 제거 (부모 컴포넌트에서 처리)
      if (onRunDeleted) {
        onRunDeleted(run);
      }
      
      // 삭제 후 모든 런을 접어서 혼란 방지
      setExpandedRuns({});
    } catch (error) {
      console.error('[ERROR] Failed to delete run:', error);
      alert(`런 삭제에 실패했습니다: ${error}`);
    }
  };

  const handleExportCSV = async () => {
    try {
      console.log('[INFO] Starting CSV export for date:', selectedDate);
      await invoke('export_daily_analysis', {
        selectedDate: selectedDate,
        format: 'csv'
      });
      console.log('[INFO] CSV export completed successfully');
    } catch (error) {
      console.error('[ERROR] Failed to export CSV:', error);
      alert(`CSV 내보내기에 실패했습니다: ${error}`);
    }
  };

  // 성능 트렌드 계산 (현재 사용되지 않음)
  // const performanceTrend = filteredDfDaily.length > 1 ? 
  //   ((filteredDfDaily[filteredDfDaily.length - 1]['실수익'] - filteredDfDaily[0]['실수익']) / filteredDfDaily[0]['실수익'] * 100) : 0;

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

      {/* Runs Table */}
      <div className="data-table-container">
        <div className="data-table-header">
          <h3 className="table-title">🚀 런 상세 정보</h3>
          <div className="table-actions">
            <button className="export-btn" onClick={handleExportCSV}>
              <span>📤</span>
              CSV 내보내기
            </button>
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
                    <div className="ship-class-badge">
                      <ShipClassIcon 
                        shipClass={run['함급'] || 1} 
                        size={16} 
                      />
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
                      <div className="timeline-item">
                        <div className="timeline-icon">🚢</div>
                        <div className="timeline-content">
                          <div className="timeline-title">함급</div>
                          <div className="timeline-time">{getShipClassDisplay(run['함급'] || 1)}</div>
                        </div>
                      </div>
                      <div className="timeline-item">
                        <button 
                          className="delete-run-btn"
                          onClick={() => handleDeleteRun(run)}
                          title="이 런을 삭제합니다"
                        >
                          🗑️ 삭제
                        </button>
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

      {/* Performance Chart */}
      <div className="chart-container">
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
  );
};

export default DailyStatsDisplay;