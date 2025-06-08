import { useState, useCallback, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";

import TitleBar from "./components/TitleBar";
import StatsDisplay from "./components/StatsDisplay";
import DailySummary from "./components/DailySummary";
import Settings from "./components/Settings";
import NotifierPopup from "./components/NotifierPopup";
import UpdateDialog from "./components/UpdateDialog";
import InitialSetup from "./components/InitialSetup";
import LoadingProgress from "./components/LoadingProgress";
import "./App.css";
import { usePopup } from "./hooks/usePopup";
import { useAbyssalData } from "./hooks/useAbyssalData";
import { useUpdater } from "./hooks/useUpdater";
import { useAppInit } from "./hooks/useAppInit";

type ViewType = 'daily-summary' | 'daily-list' | 'overall' | 'settings';

function App() {
  const [activeView, setActiveView] = useState<ViewType>('daily-list');
  const [isDailySubMenuOpen, setIsDailySubMenuOpen] = useState(true);

  const [selectedDate, setSelectedDate] = useState<string>('');

  const { triggerPopup, popupProps } = usePopup();
  
  const { 
    abyssalData, 
    dataLoading, 
    loadingSteps, 
    loadAbyssalData, 
    lightRefreshAbyssalData, 
    handleRunDeleted,
    updateLoadingStep
  } = useAbyssalData(triggerPopup);

  const {
    showUpdateDialog,
    latestVersion,
    currentVersion,
    isDownloadingUpdate,
    downloadProgress,
    handleUpdate,
    handleCloseUpdateDialog,
    checkForUpdates,
    setCurrentVersion
  } = useUpdater(triggerPopup);

  const {
    appInitializing,
    needsInitialSetup,
    checkingConfig,
    handleInitialSetupComplete
  } = useAppInit({
    updateLoadingStep,
    loadAbyssalData,
    checkForUpdates,
    setCurrentVersion,
    triggerPopup,
  });

  const handleRefresh = useCallback(() => {
    return loadAbyssalData();
  }, [loadAbyssalData]);



  useEffect(() => {
    const unlisten = listen("abyssal_run_completed", () => {
      console.log('[INFO] New abyssal run detected, triggering light refresh...');
      lightRefreshAbyssalData();
    });

    return () => {
      unlisten.then(f => f());
    };
  }, [lightRefreshAbyssalData]);



  // selectedDate 초기화
  useEffect(() => {
    if (abyssalData && abyssalData.daily_stats && Object.keys(abyssalData.daily_stats).length > 0 && !selectedDate) {
      const dates = Object.keys(abyssalData.daily_stats).sort().reverse();
      const latestDate = dates[0];
      setSelectedDate(latestDate);
    }
  }, [abyssalData, selectedDate]);

  if (checkingConfig) {
    return (
      <div className="app-container" data-tauri-drag-region>
        <TitleBar />
        <div className="loading-container">
          <p>설정 확인 중...</p>
        </div>
      </div>
    );
  }

  if (needsInitialSetup) {
    return (
      <div className="app-container" data-tauri-drag-region>
        <TitleBar />
        <InitialSetup onSetupComplete={handleInitialSetupComplete} />
      </div>
    );
  }

  if (appInitializing || dataLoading) {
    return (
      <div className="app-container" data-tauri-drag-region>
        <TitleBar />
        <LoadingProgress steps={loadingSteps} />
      </div>
    );
  }

  return (
    <div className="app-container" data-tauri-drag-region>
      <TitleBar />
      
      <div className="content-area">
        <nav className="sidebar">
          <div className="sidebar-main-actions">
            <button 
              className={`nav-button ${isDailySubMenuOpen ? 'active' : ''}`}
              onClick={() => {
                setIsDailySubMenuOpen(true);
                setActiveView('daily-list');
              }}
              title="일별 분석"
            >
              <i className="fas fa-calendar-day"></i>
            </button>
            
            {isDailySubMenuOpen && (
              <div className="sidebar-sub-menu">
                <button 
                  className={`nav-button sub-button ${activeView === 'daily-list' ? 'active' : ''}`} 
                  onClick={() => setActiveView('daily-list')}
                  title="상세 런 목록"
                >
                  <i className="fas fa-list-ul"></i>
                </button>
                <button 
                  className={`nav-button sub-button ${activeView === 'daily-summary' ? 'active' : ''}`} 
                  onClick={() => setActiveView('daily-summary')}
                  title="일별 요약"
                >
                  <i className="fas fa-chart-line"></i>
                </button>
              </div>
            )}

            <button 
              className={`nav-button ${activeView === 'overall' ? 'active' : ''}`} 
              onClick={() => {
                setActiveView('overall');
                setIsDailySubMenuOpen(false);
              }}
              title="전체 통계"
            >
              <i className="fas fa-chart-pie"></i>
            </button>
            <button 
              className={`nav-button ${activeView === 'settings' ? 'active' : ''}`} 
              onClick={() => {
                setActiveView('settings');
                setIsDailySubMenuOpen(false);
              }}
              title="설정"
            >
              <i className="fas fa-cog"></i>
            </button>
          </div>
          <div className="sidebar-extra-actions">
            <button 
              onClick={handleRefresh} 
              disabled={dataLoading} 
              className="nav-button" 
              title="새로고침"
            >
              {dataLoading ? (
                <i className="fas fa-spinner fa-spin"></i>
              ) : (
                <i className="fas fa-redo"></i>
              )}
            </button>
          </div>
        </nav>
        
        <main className="main-content">
          {(activeView === 'daily-list') && abyssalData && (
            <StatsDisplay 
              data={abyssalData}
              initialView='daily'
              selectedDate={selectedDate}
              setSelectedDate={setSelectedDate}
              onRunDeleted={handleRunDeleted}
            />
          )}
          {activeView === 'daily-summary' && abyssalData && (
            <DailySummary
              df={abyssalData.df}
              daily_stats={abyssalData.daily_stats}
              item_buy_price_cache={abyssalData.item_buy_price_cache}
              selectedDate={selectedDate}
              setSelectedDate={setSelectedDate}
            />
          )}
          {activeView === 'overall' && abyssalData && (
            <StatsDisplay 
              data={abyssalData}
              initialView='overall'
              selectedDate={selectedDate}
              setSelectedDate={setSelectedDate}
              onRunDeleted={handleRunDeleted}
            />
          )}
          {activeView === 'settings' && (
            <Settings onSettingsSaved={handleRefresh} triggerPopup={triggerPopup} />
          )}
        </main>
      </div>

      <NotifierPopup {...popupProps} />

      {showUpdateDialog && (
        <UpdateDialog
          currentVersion={currentVersion}
          latestVersion={latestVersion}
          onUpdate={handleUpdate}
          onClose={handleCloseUpdateDialog}
          isDownloading={isDownloadingUpdate}
          downloadProgress={downloadProgress}
        />
      )}
    </div>
  );
}

export default App;
