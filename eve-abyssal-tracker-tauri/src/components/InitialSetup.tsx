import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import './InitialSetup.css';

interface InitialSetupProps {
  onSetupComplete: () => void;
}

const InitialSetup: React.FC<InitialSetupProps> = ({ onSetupComplete }) => {
  const [logPath, setLogPath] = useState('');
  const [characterName, setCharacterName] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');

  const handleBrowseLogPath = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'EVE 로그 폴더 선택',
      });

      if (selected && typeof selected === 'string') {
        setLogPath(selected);
      }
    } catch (err) {
      console.error('Failed to browse log path:', err);
      setError('폴더 선택 중 오류가 발생했습니다.');
    }
  };

  const handleSubmit = async () => {
    if (!logPath.trim()) {
      setError('로그 파일 경로를 선택해주세요.');
      return;
    }
    
    if (!characterName.trim()) {
      setError('캐릭터 이름을 입력해주세요.');
      return;
    }

    setIsLoading(true);
    setError('');

    try {
      // 설정 저장
      await invoke('set_log_path', { path: logPath.trim() });
      await invoke('set_character_name', { characterName: characterName.trim() });
      
      // 설정 완료 후 메인 화면으로 이동
      onSetupComplete();
    } catch (err) {
      console.error('Failed to save settings:', err);
      setError(`설정 저장 중 오류가 발생했습니다: ${err}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleSkip = () => {
    // 임시로 기본값 설정하고 넘어가기
    onSetupComplete();
  };

  return (
    <div className="initial-setup-overlay">
      <div className="initial-setup-container">
        <div className="setup-header">
          <div className="setup-icon">🚀</div>
          <h1 className="setup-title">EVE Abyssal Tracker 설정</h1>
          <p className="setup-subtitle">
            처음 사용하시는군요! 트래커를 사용하기 위해 몇 가지 설정이 필요합니다.
          </p>
        </div>

        <div className="setup-content">
          <div className="setup-section">
            <div className="section-header">
              <h3 className="section-title">
                <span className="section-icon">📁</span>
                EVE 로그 폴더 경로
              </h3>
              <p className="section-description">
                EVE Online의 채팅 로그가 저장되는 폴더를 선택해주세요.
                <br />
                일반적으로 <code>C:\Users\[사용자명]\Documents\EVE\logs\Chatlogs</code> 위치에 있습니다.
              </p>
            </div>
            <div className="input-group">
              <input
                type="text"
                value={logPath}
                onChange={(e) => setLogPath(e.target.value)}
                placeholder="로그 폴더 경로를 선택하거나 직접 입력해주세요..."
                className="setup-input"
                disabled={isLoading}
              />
              <button
                onClick={handleBrowseLogPath}
                className="browse-button"
                disabled={isLoading}
              >
                <span className="button-icon">📂</span>
                폴더 선택
              </button>
            </div>
          </div>

          <div className="setup-section">
            <div className="section-header">
              <h3 className="section-title">
                <span className="section-icon">👤</span>
                캐릭터 이름
              </h3>
              <p className="section-description">
                추적하고자 하는 EVE Online 캐릭터의 이름을 입력해주세요.
                <br />
                로그 파일에서 해당 캐릭터의 어비셜 활동을 분석합니다.
              </p>
            </div>
            <div className="input-group">
              <input
                type="text"
                value={characterName}
                onChange={(e) => setCharacterName(e.target.value)}
                placeholder="캐릭터 이름을 입력해주세요..."
                className="setup-input"
                disabled={isLoading}
              />
            </div>
          </div>

          {error && (
            <div className="error-message">
              <span className="error-icon">⚠️</span>
              {error}
            </div>
          )}
        </div>

        <div className="setup-footer">
          <button
            onClick={handleSkip}
            className="secondary-button"
            disabled={isLoading}
          >
            나중에 설정
          </button>
          <button
            onClick={handleSubmit}
            className="primary-button"
            disabled={isLoading}
          >
            {isLoading ? (
              <>
                <span className="loading-spinner">⏳</span>
                설정 저장 중...
              </>
            ) : (
              <>
                <span className="button-icon">✅</span>
                설정 완료
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  );
};

export default InitialSetup; 