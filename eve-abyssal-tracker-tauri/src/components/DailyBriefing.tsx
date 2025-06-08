import React from 'react';
import './DailyBriefing.css';

interface DailyBriefingProps {
  stats: {
    total_runs: number;
    total_profit: number;
    total_loot_value: number;
    total_used_value: number;
  } | undefined;
}

const DailyBriefing: React.FC<DailyBriefingProps> = ({ stats }) => {
  if (!stats) {
    return (
      <div className="daily-briefing-container">
        <p>해당 날짜의 데이터가 없습니다.</p>
      </div>
    );
  }

  return (
    <div className="daily-briefing-container">
      <div className="briefing-item">
        <span className="briefing-label">총 런 수</span>
        <span className="briefing-value">{stats.total_runs}</span>
      </div>
      <div className="briefing-item">
        <span className="briefing-label">총 수익</span>
        <span className="briefing-value">{stats.total_profit.toLocaleString()} ISK</span>
      </div>
      <div className="briefing-item">
        <span className="briefing-label">총 전리품 가치</span>
        <span className="briefing-value">{stats.total_loot_value.toLocaleString()} ISK</span>
      </div>
      <div className="briefing-item">
        <span className="briefing-label">총 사용 비용</span>
        <span className="briefing-value">{stats.total_used_value.toLocaleString()} ISK</span>
      </div>
    </div>
  );
};

export default DailyBriefing; 