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
    console.log("[DEBUG] triggerPopup called with:", { title, message, type });
    setPopupTitle(title);
    setPopupMessage(message);
    setPopupType(type);
    setShowPopup(true);
    console.log("[DEBUG] Popup show state set to true");
    setTimeout(() => {
      console.log("[DEBUG] Auto-hiding popup after 5 seconds");
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
      console.log("[DEBUG] 이미 데이터 로딩 중이므로 중복 호출을 건너뜁니다.");
      return;
    }
    
    isDataLoadingRef.current = true;
    setDataLoading(true);
    setDataError(null);
    resetLoadingSteps();
    
    try {
      const parsedResult = await invoke("analyze_abyssal_data_command") as AbyssalData;
      console.log("[DEBUG] 로딩된 데이터:", parsedResult);
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



  useEffect(() => {
    const unlistenPopup = listen("trigger_popup", (event) => {
      console.log("[DEBUG] Popup event received:", JSON.stringify(event.payload, null, 2));
      const payload = event.payload as { title: string; message: string; type?: "info" | "warning" | "error" };
      console.log("[DEBUG] Calling triggerPopup with:", payload.title, payload.message, payload.type);
      triggerPopup(payload.title, payload.message, payload.type);
    });

    const unlistenLogMonitorStatus = listen("log_monitor_status", (event) => {
      const payload = event.payload as { status: string };
      setLogMonitorRunning(payload.status === "started");
    });

    const unlistenAbyssalRunCompleted = listen("abyssal_run_completed", () => {
      console.log("[DEBUG] Abyssal run completed event received - window should be opened automatically");
      // 새 윈도우는 백엔드에서 자동으로 열림
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
  }, [triggerPopup, loadAbyssalData]);

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
