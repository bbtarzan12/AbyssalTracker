import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import TitleBar from "./components/TitleBar";
import StatsDisplay from "./components/StatsDisplay";
import Settings from "./components/Settings";
import NotifierPopup from "./components/NotifierPopup";
import UpdateDialog from "./components/UpdateDialog";
import InitialSetup from "./components/InitialSetup";

import LoadingProgress from "./components/LoadingProgress";
import "./App.css";
import type { RunData, AbyssalData } from "./types";

interface LoadingStep {
  id: string;
  name: string;
  status: 'pending' | 'loading' | 'completed' | 'error';
  progress?: number;
  message?: string;
}

interface LoadingProgressEvent {
  step: string;
  message: string;
  progress?: number;
  completed: boolean;
}

function App() {
  const [activeTab, setActiveTab] = useState<'stats' | 'settings'>('stats');
  const [showPopup, setShowPopup] = useState(false);
  const [popupTitle, setPopupTitle] = useState("");
  const [popupMessage, setPopupMessage] = useState("");
  const [popupType, setPopupType] = useState<"info" | "warning" | "error">("info");
  const [logMonitorRunning, setLogMonitorRunning] = useState(false);
  const [appInitializing, setAppInitializing] = useState(true);
  const [needsInitialSetup, setNeedsInitialSetup] = useState(false);
  const [checkingConfig, setCheckingConfig] = useState(true);
  
  // 업데이트 관련 상태
  const [showUpdateDialog, setShowUpdateDialog] = useState(false);
  const [latestVersion, setLatestVersion] = useState("");
  const [currentVersion, setCurrentVersion] = useState("");
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const [isDownloadingUpdate, setIsDownloadingUpdate] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(0);
  

  
  // 데이터 관련 상태
  const [abyssalData, setAbyssalData] = useState<AbyssalData | null>(null);
  const [dataLoading, setDataLoading] = useState(false);
  const [dataError, setDataError] = useState<string | null>(null);
  const [loadingSteps, setLoadingSteps] = useState<LoadingStep[]>([
    { id: 'csv_load', name: 'CSV 파일 로드', status: 'pending' },
    { id: 'item_collection', name: '아이템 이름 수집', status: 'pending' },
    { id: 'type_id_fetch', name: 'ESI API 아이템 변환', status: 'pending' },
    { id: 'price_fetch', name: 'Fuzzwork 시세 조회', status: 'pending' },
    { id: 'analysis', name: '데이터 분석 및 통계 생성', status: 'pending' },
  ]);
  const isDataLoadingRef = useRef(false);

  const triggerPopup = useCallback((title: string, message: string, type: "info" | "warning" | "error" = "info") => {
    setPopupTitle(title);
    setPopupMessage(message);
    setPopupType(type);
    setShowPopup(true);
    setTimeout(() => {
      setShowPopup(false);
    }, 5000);
  }, []);


  // 업데이트 실행 함수
  const handleUpdate = useCallback(async () => {
    setIsDownloadingUpdate(true);
    setDownloadProgress(0);
    
    try {
      // 가짜 진행률 표시 (실제 다운로드 진행률은 백엔드에서 구현하기 복잡하므로)
      const progressInterval = setInterval(() => {
        setDownloadProgress(prev => {
          if (prev >= 95) return prev;
          return prev + Math.random() * 10;
        });
      }, 200);
      
      await invoke("download_and_install_update_command");
      
      clearInterval(progressInterval);
      setDownloadProgress(100);
      
      // 즉시 다이얼로그 닫기 (설치 프로그램이 시작되면 앱이 곧바로 종료됨)
      setShowUpdateDialog(false);
      
    } catch (error) {
      console.error("업데이트 실행 실패:", error);
      setIsDownloadingUpdate(false);
      setDownloadProgress(0);
      triggerPopup("업데이트 실패", `업데이트 중 오류가 발생했습니다: ${error}`, "error");
    }
  }, [triggerPopup]);

  // 업데이트 다이얼로그 닫기
  const handleCloseUpdateDialog = useCallback(() => {
    setShowUpdateDialog(false);
    setIsDownloadingUpdate(false);
    setDownloadProgress(0);
    // 데이터는 이미 로딩되었으므로 별도 작업 불필요
  }, []);

  const resetLoadingSteps = useCallback(() => {
    setLoadingSteps([
      { id: 'csv_load', name: 'CSV 파일 로드', status: 'pending' },
      { id: 'item_collection', name: '아이템 이름 수집', status: 'pending' },
      { id: 'type_id_fetch', name: 'ESI API 아이템 변환', status: 'pending' },
      { id: 'price_fetch', name: 'Fuzzwork 시세 조회', status: 'pending' },
      { id: 'analysis', name: '데이터 분석 및 통계 생성', status: 'pending' },
    ]);
  }, []);

  const loadAbyssalData = useCallback(async () => {
    if (isDataLoadingRef.current) {
      return;
    }
    
    isDataLoadingRef.current = true;
    setDataLoading(true);
    setDataError(null);
    resetLoadingSteps();
    
    try {
      const parsedResult = await invoke("analyze_abyssal_data_command") as AbyssalData;
      setAbyssalData(parsedResult);
      
      // 분석 완료 후 IconCache 다시 로드
      try {
        await invoke("reload_icon_cache");
        console.log('[INFO] IconCache reloaded after data analysis');
      } catch (cacheError) {
        console.warn('[WARN] Failed to reload icon cache:', cacheError);
      }
    } catch (err) {
      console.error("Failed to fetch abyssal data:", err);
      setDataError(`데이터 로딩 실패: ${err}`);
      triggerPopup("데이터 로딩 실패", `데이터를 불러오는 중 오류가 발생했습니다: ${err}`, "error");
      
      // 에러 시 모든 단계를 에러 상태로 설정
      setLoadingSteps(prev => prev.map(step => ({ ...step, status: 'error' as const })));
    } finally {
      setDataLoading(false);
      isDataLoadingRef.current = false;
    }
  }, [triggerPopup, resetLoadingSteps]);

  // 가벼운 데이터 새로고침 (로딩창 없음)
  const lightRefreshAbyssalData = useCallback(async () => {
    if (isDataLoadingRef.current) {
      return;
    }
    
    isDataLoadingRef.current = true;
    setDataError(null);
    
    try {
      const parsedResult = await invoke("light_refresh_abyssal_data_command") as AbyssalData;
      setAbyssalData(parsedResult);
      
      // 가벼운 새로고침 후에도 IconCache 다시 로드 (새로운 아이템이 있을 수 있음)
      try {
        await invoke("reload_icon_cache");
        console.log('[INFO] IconCache reloaded after light refresh');
      } catch (cacheError) {
        console.warn('[WARN] Failed to reload icon cache after light refresh:', cacheError);
      }
    } catch (err) {
      console.error("Failed to light refresh abyssal data:", err);
      setDataError(`가벼운 데이터 새로고침 실패: ${err}`);
      triggerPopup("데이터 새로고침 실패", `데이터를 새로고침하는 중 오류가 발생했습니다: ${err}`, "error");
    } finally {
      isDataLoadingRef.current = false;
    }
  }, [triggerPopup]);

  // 런 삭제 후 UI 상태에서만 제거 (API 호출 없음)
  const handleRunDeleted = useCallback((deletedRun: RunData) => {
    if (!abyssalData) return;

    const newData = { ...abyssalData };
    
    // df에서 삭제
    newData.df = newData.df.filter(run => 
      !(run['시작시각(KST)'] === deletedRun['시작시각(KST)'] && 
        run['종료시각(KST)'] === deletedRun['종료시각(KST)'])
    );
    
    // daily_stats에서 삭제
    const runDate = deletedRun['날짜'];
    if (newData.daily_stats[runDate]) {
      const filteredRuns = newData.daily_stats[runDate].runs.filter(run =>
        !(run['시작시각(KST)'] === deletedRun['시작시각(KST)'] && 
          run['종료시각(KST)'] === deletedRun['종료시각(KST)'])
      );
      
      if (filteredRuns.length === 0) {
        // 해당 날짜의 런이 모두 없어지면 삭제
        delete newData.daily_stats[runDate];
      } else {
        // 통계 재계산
        const avg_isk = filteredRuns.reduce((sum, run) => sum + run['실수익'], 0) / filteredRuns.length;
        const avg_time = filteredRuns.reduce((sum, run) => sum + run['런 소요(분)'], 0) / filteredRuns.length;
        const avg_iskph = filteredRuns.reduce((sum, run) => sum + run['ISK/h'], 0) / filteredRuns.length;
        
        newData.daily_stats[runDate] = {
          runs: filteredRuns,
          avg_isk,
          avg_time,
          avg_iskph,
        };
      }
    }
    
    // overall_stats 재계산
    if (newData.df.length > 0) {
      newData.overall_stats.avg_isk = newData.df.reduce((sum, run) => sum + run['실수익'], 0) / newData.df.length;
      newData.overall_stats.avg_time = newData.df.reduce((sum, run) => sum + run['런 소요(분)'], 0) / newData.df.length;
      newData.overall_stats.avg_iskph = newData.df.reduce((sum, run) => sum + run['ISK/h'], 0) / newData.df.length;
      
      // tier_weather_stats 재계산
      const tierWeatherGroups: { [key: string]: RunData[] } = {};
      newData.df.forEach(run => {
        const parts = run['어비셜 종류'].split(' ');
        if (parts.length >= 2) {
          const key = `${parts[0]} ${parts[1]}`;
          if (!tierWeatherGroups[key]) tierWeatherGroups[key] = [];
          tierWeatherGroups[key].push(run);
        }
      });
      
      newData.overall_stats.tier_weather_stats = Object.entries(tierWeatherGroups).map(([key, runs]) => {
        const [tier, weather] = key.split(' ');
        return {
          tier,
          weather,
          runs_count: runs.length,
          avg_isk: runs.reduce((sum, run) => sum + run['실수익'], 0) / runs.length,
          avg_time: runs.reduce((sum, run) => sum + run['런 소요(분)'], 0) / runs.length,
          avg_iskph: runs.reduce((sum, run) => sum + run['ISK/h'], 0) / runs.length,
        };
      });
    } else {
      // 모든 런이 삭제된 경우
      newData.overall_stats = {
        avg_isk: 0,
        avg_time: 0,
        avg_iskph: 0,
        tier_weather_stats: [],
      };
    }
    
    setAbyssalData(newData);
    console.log('[INFO] UI state updated after run deletion');
  }, [abyssalData]);

  // 자동 모니터링 시작 함수
  const startLogMonitoring = useCallback(async () => {
    try {
      console.log('[INFO] 자동으로 로그 모니터링을 시작합니다...');
      await invoke("start_log_monitor_command");
      setLogMonitorRunning(true);
      triggerPopup("모니터링 시작", "EVE 로그 모니터링이 자동으로 시작되었습니다.", "info");
    } catch (error) {
      console.error("자동 모니터링 시작 실패:", error);
      triggerPopup("모니터링 시작 실패", `자동 모니터링 시작에 실패했습니다: ${error}`, "warning");
    }
  }, [triggerPopup]);

  // 초기 설정 완료 후 데이터 로딩 및 모니터링 시작
  const handleSetupComplete = useCallback(async () => {
    setNeedsInitialSetup(false);
    setAppInitializing(true);
    try {
      await loadAbyssalData();
      // 데이터 로딩 완료 후 자동으로 모니터링 시작
      await startLogMonitoring();
    } catch (error) {
      console.error("데이터 로딩 실패:", error);
    } finally {
      setAppInitializing(false);
    }
  }, [loadAbyssalData, startLogMonitoring]);

  useEffect(() => {
    const unlistenPopup = listen("trigger_popup", (event) => {
      const payload = event.payload as { title: string; message: string; type?: "info" | "warning" | "error" };
      triggerPopup(payload.title, payload.message, payload.type);
    });

    const unlistenLogMonitorStatus = listen("log_monitor_status", (event) => {
      const payload = event.payload as { status: string };
      setLogMonitorRunning(payload.status === "started");
    });

    const unlistenAbyssalRunCompleted = listen("abyssal_run_completed", () => {
      // 새 런이 추가되면 자동으로 가벼운 데이터 새로고침
      console.log('[INFO] New abyssal run detected, triggering light refresh...');
      lightRefreshAbyssalData();
    });

    // 로딩 진행 상황 이벤트 수신
    const unlistenProgress = listen<LoadingProgressEvent>("loading_progress", (event) => {
      const { step, message, progress, completed } = event.payload;
      
      setLoadingSteps(prev => prev.map(loadingStep => {
        if (loadingStep.id === step) {
          return {
            ...loadingStep,
            status: completed ? 'completed' : 'loading',
            progress: progress,
            message: message,
          };
        }
        return loadingStep;
      }));
    });

    // 현재 버전 가져오기
    const loadCurrentVersion = async () => {
      try {
        const version = await invoke("get_current_version") as string;
        setCurrentVersion(version);
        console.log('[INFO] Current version loaded:', version);
      } catch (error) {
        console.error("Failed to get current version:", error);
        setCurrentVersion("Unknown");
      }
    };

    // 설정 확인 함수
    const checkInitialConfig = async () => {
      try {
        const config = await invoke("get_config") as any;
        const hasLogPath = config.general.log_path && config.general.log_path.trim() !== '';
        const hasCharacterName = config.general.character_name && config.general.character_name.trim() !== '';
        
        if (!hasLogPath || !hasCharacterName) {
          setNeedsInitialSetup(true);
          setAppInitializing(false);
        } else {
          setNeedsInitialSetup(false);
          await loadAbyssalData();
          // 기존 설정이 있는 경우 데이터 로딩 완료 후 자동으로 모니터링 시작
          await startLogMonitoring();
          setAppInitializing(false);
        }
      } catch (error) {
        console.error("설정 확인 실패:", error);
        setNeedsInitialSetup(true);
        setAppInitializing(false);
      } finally {
        setCheckingConfig(false);
      }
    };

  // 앱 초기화 및 데이터 로딩 처리
    const initializeApp = async () => {
      try {
        // 1. 먼저 현재 버전 로드
        await loadCurrentVersion();
        
        // 2. 설정 확인
        await checkInitialConfig();
        
        // 3. 설정이 완료된 경우에만 백그라운드에서 업데이트 체크
        if (!needsInitialSetup && !checkingUpdate) {
          setCheckingUpdate(true);
          invoke("check_for_update_command")
            .then((updateInfo: any) => {
              if (updateInfo.available) {
                setLatestVersion(updateInfo.latest_version);
                setShowUpdateDialog(true);
              }
            })
            .catch((error) => {
              console.error("업데이트 체크 실패:", error);
              // 업데이트 체크 실패는 조용히 처리
            })
            .finally(() => {
              setCheckingUpdate(false);
            });
        }
      } catch (error) {
        console.error("앱 초기화 실패:", error);
        setAppInitializing(false);
        setCheckingConfig(false);
      }
    };

    initializeApp();

    // 컴포넌트 언마운트 시 리스너 해제
    return () => {
      unlistenPopup.then(f => f());
      unlistenLogMonitorStatus.then(f => f());
      unlistenAbyssalRunCompleted.then(f => f());
      unlistenProgress.then(f => f());
    };
  }, [triggerPopup, loadAbyssalData, lightRefreshAbyssalData, startLogMonitoring]);

  // 초기 설정이 필요한 경우
  if (needsInitialSetup) {
    return <InitialSetup onSetupComplete={handleSetupComplete} />;
  }

  // 앱 초기화 중이거나 데이터 로딩 중인 경우
  if (appInitializing || dataLoading || checkingConfig) {
    return (
      <LoadingProgress 
        show={true} 
        steps={loadingSteps}
        title={checkingConfig ? "설정 확인 중..." : appInitializing ? "EVE Abyssal Tracker 시작 중..." : "데이터 로딩 및 분석 중..."}
      />
    );
  }

  return (
    <>
      <NotifierPopup
        show={showPopup}
        title={popupTitle}
        message={popupMessage}
        type={popupType}
        onClose={() => setShowPopup(false)}
      />
      <UpdateDialog
        show={showUpdateDialog}
        latestVersion={latestVersion}
        currentVersion={currentVersion}
        onClose={handleCloseUpdateDialog}
        onUpdate={handleUpdate}
        isDownloading={isDownloadingUpdate}
        downloadProgress={downloadProgress}
      />
      <div className="app-container">
        <TitleBar />

        <header className="tab-container">
        <div className="nav-wrapper">
          <div className="nav-brand">
            <div className="nav-brand-icon">⚡</div>
            EVE 어비셜 트래커
          </div>
          <div className="nav-tabs">
            <button
              onClick={() => setActiveTab('stats')}
              className={`tab-button ${activeTab === 'stats' ? 'active' : ''}`}
            >
              <span>📊</span>
              분석 대시보드
            </button>
            <button
              onClick={() => setActiveTab('settings')}
              className={`tab-button ${activeTab === 'settings' ? 'active' : ''}`}
            >
              <span>⚙️</span>
              설정
            </button>
          </div>
          <div className="nav-actions">
            <button onClick={loadAbyssalData} className="toolbar-btn refresh-btn" title="데이터 새로고침">
              <span className="btn-icon">🔄</span>
              <span className="btn-text">새로고침</span>
            </button>
          </div>
        </div>
      </header>
      <main>
        {activeTab === 'stats' && (
          <StatsDisplay
            data={abyssalData}
            dataError={dataError}
            onRefresh={loadAbyssalData}
            onLightRefresh={lightRefreshAbyssalData}
            onRunDeleted={handleRunDeleted}
            triggerPopup={triggerPopup}
          />
        )}
        {activeTab === 'settings' && (
          <Settings
            logMonitorRunning={logMonitorRunning}
            setLogMonitorRunning={setLogMonitorRunning}
            triggerPopup={triggerPopup}
          />
        )}
      </main>
      </div>
    </>
  );
}

export default App;
