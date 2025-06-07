import React, { useState, useEffect } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { getVersion } from '@tauri-apps/api/app';
import './TitleBar.css';

const TitleBar: React.FC = () => {
  const [isMaximized, setIsMaximized] = useState(false);
  const [appVersion, setAppVersion] = useState('');

  useEffect(() => {
    const checkMaximized = async () => {
      const window = getCurrentWindow();
      const maximized = await window.isMaximized();
      setIsMaximized(maximized);
    };

    const getAppVersion = async () => {
      try {
        const version = await getVersion();
        setAppVersion(version);
      } catch (error) {
        console.error('Failed to get app version:', error);
        setAppVersion('Unknown'); // fallback - should not happen
      }
    };

    checkMaximized();
    getAppVersion();
  }, []);

  const handleMinimize = async () => {
    const window = getCurrentWindow();
    await window.minimize();
  };

  const handleMaximize = async () => {
    const window = getCurrentWindow();
    await window.toggleMaximize();
    setIsMaximized(!isMaximized);
  };

  const handleClose = async () => {
    const window = getCurrentWindow();
    await window.close();
  };

  const handleDragStart = async () => {
    const window = getCurrentWindow();
    await window.startDragging();
  };

  return (
    <div className="titlebar">
      <div className="titlebar-drag-region" onMouseDown={handleDragStart}>
        <div className="titlebar-content">
          <div className="titlebar-icon">
            <span className="app-icon">🚀</span>
          </div>
          <div className="titlebar-title">
            EVE Abyssal Tracker v{appVersion}
          </div>
        </div>
      </div>
      
      <div className="titlebar-controls">
        <button 
          className="titlebar-button minimize-button"
          onClick={handleMinimize}
          title="최소화"
        >
          <svg width="10" height="10" viewBox="0 0 10 10">
            <rect x="2" y="4.5" width="6" height="1" fill="currentColor"/>
          </svg>
        </button>
        
        <button 
          className="titlebar-button maximize-button"
          onClick={handleMaximize}
          title={isMaximized ? "복원" : "최대화"}
        >
          {isMaximized ? (
            <svg width="10" height="10" viewBox="0 0 10 10">
              <rect x="2" y="3" width="5" height="5" stroke="currentColor" strokeWidth="1" fill="none"/>
              <rect x="3" y="2" width="5" height="5" stroke="currentColor" strokeWidth="1" fill="none"/>
            </svg>
          ) : (
            <svg width="10" height="10" viewBox="0 0 10 10">
              <rect x="2" y="2" width="6" height="6" stroke="currentColor" strokeWidth="1" fill="none"/>
            </svg>
          )}
        </button>
        
        <button 
          className="titlebar-button close-button"
          onClick={handleClose}
          title="닫기"
        >
          <svg width="10" height="10" viewBox="0 0 10 10">
            <line x1="2" y1="2" x2="8" y2="8" stroke="currentColor" strokeWidth="1"/>
            <line x1="8" y1="2" x2="2" y2="8" stroke="currentColor" strokeWidth="1"/>
          </svg>
        </button>
      </div>
    </div>
  );
};

export default TitleBar; 