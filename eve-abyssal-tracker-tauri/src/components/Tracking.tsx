import React from 'react';
import { Dispatch, SetStateAction } from 'react';
import { invoke } from "@tauri-apps/api/core";
import './Tracking.css';

interface TrackingProps {
  logMonitorRunning: boolean;
  setLogMonitorRunning: Dispatch<SetStateAction<boolean>>;
  triggerPopup: (title: string, message: string, type?: "info" | "warning" | "error") => void;
}

const Tracking: React.FC<TrackingProps> = ({ logMonitorRunning, setLogMonitorRunning, triggerPopup }) => {

  const handleStartTracking = async () => {
    try {
      await invoke("start_log_monitor_command");
      setLogMonitorRunning(true);
      triggerPopup("트래킹 시작", "어비셜 트래킹이 시작되었습니다.", "info");
    } catch (e) {
      console.error("Failed to start tracking:", e);
      triggerPopup("트래킹 시작 실패", `트래킹 시작 중 오류 발생: ${e}`, "error");
    }
  };

  const handleStopTracking = async () => {
    try {
      await invoke("stop_log_monitor_command");
      setLogMonitorRunning(false);
      triggerPopup("트래킹 중지", "어비셜 트래킹이 중지되었습니다.", "info");
    } catch (e) {
      console.error("Failed to stop tracking:", e);
      triggerPopup("트래킹 중지 실패", `트래킹 중지 중 오류 발생: ${e}`, "error");
    }
  };

  const handleTestWindow = async () => {
    try {
      await invoke("test_abyssal_window");
      triggerPopup("테스트 완료", "어비셜 결과 윈도우 테스트가 실행되었습니다.", "info");
    } catch (e) {
      console.error("Failed to test window:", e);
      triggerPopup("테스트 실패", `윈도우 테스트 중 오류 발생: ${e}`, "error");
    }
  };

  return (
    <div className="tracking-container">
      <h1>🎯 어비셜 트래킹</h1>
      
      <div className="tracking-status">
        <h2>현재 상태: {logMonitorRunning ? '🟢 트래킹 중' : '🔴 중지됨'}</h2>
      </div>

      <div className="tracking-buttons">
        {logMonitorRunning ? (
          <button onClick={handleStopTracking} className="tracking-button stop">
            트래킹 중지
          </button>
        ) : (
          <button onClick={handleStartTracking} className="tracking-button start">
            트래킹 시작
          </button>
        )}
        
        <button onClick={handleTestWindow} className="tracking-button test">
          결과창 테스트
        </button>
      </div>
    </div>
  );
};

export default Tracking; 