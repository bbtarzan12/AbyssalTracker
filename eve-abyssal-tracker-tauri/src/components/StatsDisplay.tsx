import React, { useState, useEffect } from 'react';
import DailyStatsDisplay from './DailyStatsDisplay';
import OverallStatsDisplay from './OverallStatsDisplay';
import './StatsDisplay.css';
import type { AbyssalData, RunData } from "../types";

interface StatsDisplayProps {
  data: AbyssalData | null;
  dataError: string | null;
  onRefresh: () => Promise<void>;
  onLightRefresh?: () => Promise<void>;
  onRunDeleted?: (run: RunData) => void;
  triggerPopup: (title: string, message: string, type?: "info" | "warning" | "error") => void;
}

const StatsDisplay: React.FC<StatsDisplayProps> = ({ data, dataError, onRefresh, onLightRefresh, onRunDeleted, triggerPopup: _triggerPopup }) => {
  const [selectedDate, setSelectedDate] = useState<string>('');
  const [activeTab, setActiveTab] = useState<'daily' | 'overall'>('daily');
  const [hasInitializedDate, setHasInitializedDate] = useState<boolean>(false);

  // 처음 데이터가 로드될 때만 최신 날짜로 설정, 이후에는 사용자 선택 유지
  useEffect(() => {
    if (data && data.daily_stats && Object.keys(data.daily_stats).length > 0 && !hasInitializedDate) {
      const dates = Object.keys(data.daily_stats).sort().reverse();
      const latestDate = dates[0];
      setSelectedDate(latestDate);
      setHasInitializedDate(true);
    }
    // 사용자가 선택한 날짜가 더 이상 존재하지 않는 경우에만 최신 날짜로 변경
    else if (data && data.daily_stats && selectedDate && !data.daily_stats[selectedDate]) {
      const dates = Object.keys(data.daily_stats).sort().reverse();
      if (dates.length > 0) {
        setSelectedDate(dates[0]);
      }
    }
  }, [data, selectedDate, hasInitializedDate]);

  if (dataError) return <div className="error-message">{dataError}</div>;
  if (!data || !data.df || data.df.length === 0) {
    return <div className="no-data-message">⚠️ 분석할 어비셜 런 데이터가 없습니다. 런을 기록한 후 다시 시도해주세요.</div>;
  }

  const { df, daily_stats, overall_stats, item_buy_price_cache } = data;

  return (
    <div className="dashboard-container">      
      <div className="dashboard-nav">
        <div className="nav-tabs">
          <button
            onClick={() => setActiveTab('daily')}
            className={`nav-tab ${activeTab === 'daily' ? 'active' : ''}`}
          >
            <span className="tab-icon">📅</span>
            <span className="tab-label">일별 분석</span>
          </button>
          <button
            onClick={() => setActiveTab('overall')}
            className={`nav-tab ${activeTab === 'overall' ? 'active' : ''}`}
          >
            <span className="tab-icon">📊</span>
            <span className="tab-label">전체 통계</span>
          </button>
        </div>
      </div>

      <div className="dashboard-content">
        {activeTab === 'daily' ? (
          <DailyStatsDisplay
            df={df}
            daily_stats={daily_stats}
            item_buy_price_cache={item_buy_price_cache}
            selectedDate={selectedDate}
            setSelectedDate={setSelectedDate}
            onDataUpdate={onRefresh}
            onLightRefresh={onLightRefresh}
            onRunDeleted={onRunDeleted}
          />
        ) : (
          <OverallStatsDisplay
            df={df}
            overall_stats={overall_stats}
          />
        )}
      </div>
    </div>
  );
};

export default StatsDisplay;