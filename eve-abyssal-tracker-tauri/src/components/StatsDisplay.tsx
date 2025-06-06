import React, { useState, useEffect } from 'react';
import DailyStatsDisplay from './DailyStatsDisplay';
import OverallStatsDisplay from './OverallStatsDisplay';
import './StatsDisplay.css';

interface StatsDisplayProps {
  data: AbyssalData | null;
  dataError: string | null;
  onRefresh: () => Promise<void>;
  triggerPopup: (title: string, message: string, type?: "info" | "warning" | "error") => void;
}

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

interface OverallStats {
  avg_isk: number;
  avg_time: number;
  avg_iskph: number;
  tier_weather_stats: {
    tier: string;
    weather: string;
    runs_count: number;
    avg_isk: number;
    avg_time: number;
    avg_iskph: number;
  }[];
}

interface AbyssalData {
  df: RunData[];
  daily_stats: DailyStats;
  overall_stats: OverallStats;
  item_buy_price_cache: { [key: string]: number };
}

const StatsDisplay: React.FC<StatsDisplayProps> = ({ data, dataError, onRefresh: _, triggerPopup: __ }) => {
  const [selectedDate, setSelectedDate] = useState<string>('');
  const [activeTab, setActiveTab] = useState<'daily' | 'overall'>('daily');

  // 데이터가 변경될 때마다 선택된 날짜 업데이트
  useEffect(() => {
    if (data && data.daily_stats && Object.keys(data.daily_stats).length > 0) {
      const dates = Object.keys(data.daily_stats).sort().reverse();
      const latestDate = dates[0];
      console.log("[DEBUG] 사용 가능한 날짜들:", dates);
      console.log("[DEBUG] 선택된 날짜:", latestDate);
      setSelectedDate(latestDate);
    }
  }, [data]);

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
        <div className="nav-status">
          <span className="status-indicator"></span>
          <span className="status-text">🔴 실시간 데이터</span>
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