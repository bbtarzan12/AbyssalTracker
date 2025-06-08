import React from 'react';

interface FilterControlsProps {
  selectedDate: string;
  onDateChange: (date: string) => void;
  availableDates: string[];
}

const FilterControls: React.FC<FilterControlsProps> = ({
  selectedDate,
  onDateChange,
  availableDates,
}) => {
  return (
    <div className="filter-controls" style={{ alignSelf: 'flex-start', marginBottom: 'var(--space-6)'}}>
      <div className="filter-group">
        <label className="filter-label">📅 날짜 선택</label>
        <select
          value={selectedDate}
          onChange={(e) => onDateChange(e.target.value)}
          className="filter-select"
        >
          {availableDates.map(date => (
            <option key={date} value={date}>{date}</option>
          ))}
        </select>
      </div>
    </div>
  );
};

export default FilterControls; 