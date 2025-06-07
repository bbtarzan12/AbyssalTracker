import React, { useState } from 'react';
import { RunTypeBadge } from './utils';
import './AbyssalResultModal.css';

interface AbyssalResultModalProps {
  show: boolean;
  startTime: string;
  endTime: string;
  duration: string;
  onSave: (abyssalType: string, items: string) => void;
  onCancel: () => void;
}

const ABYSSAL_TYPES_BY_TIER = {
  "T1 (Tier 1)": ["T1 Exotic", "T1 Firestorm", "T1 Gamma", "T1 Dark", "T1 Electrical"],
  "T2 (Tier 2)": ["T2 Exotic", "T2 Firestorm", "T2 Gamma", "T2 Dark", "T2 Electrical"],
  "T3 (Tier 3)": ["T3 Exotic", "T3 Firestorm", "T3 Gamma", "T3 Dark", "T3 Electrical"],
  "T4 (Tier 4)": ["T4 Exotic", "T4 Firestorm", "T4 Gamma", "T4 Dark", "T4 Electrical"],
  "T5 (Tier 5)": ["T5 Exotic", "T5 Firestorm", "T5 Gamma", "T5 Dark", "T5 Electrical"],
  "T6 (Tier 6)": ["T6 Exotic", "T6 Firestorm", "T6 Gamma", "T6 Dark", "T6 Electrical"],
};

// 기본값을 위한 첫 번째 타입
const DEFAULT_TYPE = ABYSSAL_TYPES_BY_TIER["T1 (Tier 1)"][0];

const AbyssalResultModal: React.FC<AbyssalResultModalProps> = ({
  show,
  startTime,
  endTime,
  duration,
  onSave,
  onCancel
}) => {
  const [selectedType, setSelectedType] = useState(DEFAULT_TYPE);
  const [items, setItems] = useState('');

  if (!show) return null;

  const handleSave = () => {
    onSave(selectedType, items.trim());
    setItems(''); // 저장 후 초기화
  };

  return (
    <div className="abyssal-modal-overlay">
      <div className="abyssal-modal">
        <div className="modal-header">
          <div className="modal-title">
            <span className="title-icon">🚀</span>
            <h2>어비셜 런 완료</h2>
          </div>
          <button onClick={onCancel} className="close-button">
            <span>✕</span>
          </button>
        </div>
        
        <div className="modal-content">
          <div className="run-timeline-section">
            <h3 className="section-title">
              <span className="section-icon">⏱️</span>
              런 정보
            </h3>
            <div className="timeline-container">
              <div className="timeline-item">
                <div className="timeline-icon start">🚀</div>
                <div className="timeline-content">
                  <div className="timeline-label">시작 시간</div>
                  <div className="timeline-value">{startTime}</div>
                </div>
              </div>
              <div className="timeline-connector"></div>
              <div className="timeline-item">
                <div className="timeline-icon end">🏁</div>
                <div className="timeline-content">
                  <div className="timeline-label">완료 시간</div>
                  <div className="timeline-value">{endTime}</div>
                </div>
              </div>
            </div>
            <div className="duration-display">
              <span className="duration-label">총 소요시간</span>
              <span className="duration-value">{duration}</span>
            </div>
          </div>

          <div className="form-section">
            <h3 className="section-title">
              <span className="section-icon">🎯</span>
              어비셜 종류
            </h3>
            <div className="type-selector">
              <select
                id="abyssal-type"
                value={selectedType}
                onChange={(e) => setSelectedType(e.target.value)}
                className="modern-select"
              >
                {Object.entries(ABYSSAL_TYPES_BY_TIER).map(([tierName, types]) => (
                  <optgroup key={tierName} label={tierName}>
                    {types.map((type) => (
                      <option key={type} value={type}>
                        {type}
                      </option>
                    ))}
                  </optgroup>
                ))}
              </select>
              <div className="selected-type-preview">
                <RunTypeBadge abyssalType={selectedType} />
              </div>
            </div>
          </div>

          <div className="form-section">
            <h3 className="section-title">
              <span className="section-icon">🎁</span>
              획득 아이템
            </h3>
            <div className="textarea-container">
              <textarea
                id="acquired-items"
                value={items}
                onChange={(e) => setItems(e.target.value)}
                placeholder="EVE에서 복사한 아이템 목록을 여기에 붙여넣으세요...&#10;&#10;예시:&#10;Compressed Arkonor* 5&#10;Compressed Spodumain* 3&#10;..."
                className="modern-textarea"
                rows={6}
              />
              <div className="textarea-footer">
                <span className="item-count">
                  {items.trim() ? items.trim().split('\n').filter(line => line.trim()).length : 0}개 라인
                </span>
              </div>
            </div>
          </div>
        </div>

        <div className="modal-footer">
          <button onClick={onCancel} className="secondary-button">
            <span>✕</span>
            취소
          </button>
          <button onClick={handleSave} className="primary-button">
            <span>💾</span>
            저장하기
          </button>
        </div>
      </div>
    </div>
  );
};

export default AbyssalResultModal; 