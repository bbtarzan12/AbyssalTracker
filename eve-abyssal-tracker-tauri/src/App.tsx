import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import TitleBar from "./components/TitleBar";
import StatsDisplay from "./components/StatsDisplay";
import Settings from "./components/Settings";
import NotifierPopup from "./components/NotifierPopup";

import LoadingProgress from "./components/LoadingProgress";
import "./App.css";

// 데이터 인터페이스들
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

    // 앱 초기화 및 데이터 로딩 처리
    const initializeApp = async () => {
      try {
        await loadAbyssalData(); // 데이터 로딩
        
        // 앱 시작 시 자동 업데이트 확인
        try {
          const updateResult = await invoke("check_for_updates") as string;
          if (updateResult.includes("업데이트 가능")) {
            triggerPopup("업데이트 알림", updateResult, "info");
          }
        } catch (e) {
          console.log("업데이트 확인 건너뜀:", e);
        }
        
        setAppInitializing(false);
      } catch (error) {
        console.error("앱 초기화 실패:", error);
        setAppInitializing(false);
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
  }, [triggerPopup, loadAbyssalData, lightRefreshAbyssalData]);

  if (appInitializing || dataLoading) {
    return (
      <LoadingProgress 
        show={true} 
        steps={loadingSteps}
        title={appInitializing ? "EVE Abyssal Tracker 시작 중..." : "데이터 로딩 및 분석 중..."}
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
