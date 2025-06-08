import React, { useState, useEffect } from 'react';
import DailyStatsDisplay from './DailyStatsDisplay';
import OverallStatsDisplay from './OverallStatsDisplay';
import './StatsDisplay.css';
import type { AbyssalData, RunData } from "../types";

interface StatsDisplayProps {
  data: AbyssalData | null;
  initialView: 'daily' | 'overall';
  selectedDate: string;
  setSelectedDate: (date: string) => void;
  onRunDeleted?: (run: RunData) => void;
}

const StatsDisplay: React.FC<StatsDisplayProps> = ({ data, initialView, selectedDate, setSelectedDate, onRunDeleted }) => {
  const [activeTab, setActiveTab] = useState<'daily' | 'overall'>(initialView);

  useEffect(() => {
    setActiveTab(initialView);
  }, [initialView]);



  if (!data || !data.df || data.df.length === 0) {
    return (
        <div className="no-data-message">
            <p>⚠️ 분석할 어비셜 런 데이터가 없습니다.</p>
            <p>사이드바의 새로고침 버튼을 눌러 데이터를 불러오세요.</p>
        </div>
    );
  }

  const { df, daily_stats, overall_stats, item_buy_price_cache } = data;

  return (
    <div className="dashboard-container">      
      <div className="dashboard-content">
        {activeTab === 'daily' ? (
          <DailyStatsDisplay
            df={df}
            daily_stats={daily_stats}
            item_buy_price_cache={item_buy_price_cache}
            selectedDate={selectedDate}
            setSelectedDate={setSelectedDate}
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