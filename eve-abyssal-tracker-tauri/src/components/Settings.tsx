import React, { useState, useEffect, useCallback } from 'react';
import { Dispatch, SetStateAction } from 'react';
import './Settings.css';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { invoke } from "@tauri-apps/api/core";

interface GeneralConfig {
  log_path: string;
  character_name: string;
}

interface TrackerConfig {
  abyssal_data_path: string;
  daily_stats_path: string;
  overall_stats_path: string;
}

interface AppConfig {
  general: GeneralConfig;
  tracker: TrackerConfig;
}

interface SettingsProps {
  logMonitorRunning: boolean;
  setLogMonitorRunning: Dispatch<SetStateAction<boolean>>;
  triggerPopup: (title: string, message: string, type?: "info" | "warning" | "error") => void;
}

const Settings: React.FC<SettingsProps> = ({ logMonitorRunning, setLogMonitorRunning, triggerPopup }) => {
  const [config, setConfig] = useState<AppConfig>({
    general: {
      log_path: '',
      character_name: '',
    },
    tracker: {
      abyssal_data_path: '',
      daily_stats_path: '',
      overall_stats_path: '',
    },
  });
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [isDirty, setIsDirty] = useState(false);


  const loadConfig = useCallback(async () => {
    setLoading(true);
    try {
      const parsedResult = await invoke("get_config") as AppConfig;
      setConfig(parsedResult);
      setIsDirty(false);
    } catch (e) {
      console.error("Failed to load config:", e);
      triggerPopup("설정 로딩 실패", `설정을 불러오는 데 실패했습니다: ${e}`, "error");
    } finally {
      setLoading(false);
    }
  }, [triggerPopup]);

  const saveConfig = async () => {
    setSaving(true);
    try {
      await invoke("set_log_path", { path: config.general.log_path });
      await invoke("set_character_name", { characterName: config.general.character_name });
      triggerPopup("설정 저장 완료", "설정이 성공적으로 저장되었습니다.", "info");
      setIsDirty(false);
    } catch (e) {
      console.error("Failed to save config:", e);
      triggerPopup("저장 실패", `설정 저장에 실패했습니다: ${e}`, "error");
    } finally {
      setSaving(false);
    }
  };

  const handleOpenCsvFolder = async () => {
    try {
      const dataPath = await invoke("get_csv_data_path") as string;
      await revealItemInDir(dataPath);
      triggerPopup("폴더 열기", "CSV 데이터 폴더가 열렸습니다.", "info");
    } catch (e) {
      console.error("Failed to open CSV folder:", e);
      triggerPopup("폴더 열기 실패", `CSV 폴더 열기에 실패했습니다: ${e}`, "error");
    }
  };

  const handleStartMonitor = async () => {
    try {
      await invoke("start_log_monitor_command");
      setLogMonitorRunning(true);
      triggerPopup("모니터링 시작", "로그 모니터링이 성공적으로 시작되었습니다.", "info");
    } catch (e) {
      console.error("Failed to start log monitor:", e);
      triggerPopup("모니터링 시작 실패", `로그 모니터링 시작에 실패했습니다: ${e}`, "error");
    }
  };

  const handleStopMonitor = async () => {
    try {
      await invoke("stop_log_monitor_command");
      setLogMonitorRunning(false);
      triggerPopup("모니터링 중지", "로그 모니터링이 중지되었습니다.", "info");
    } catch (e) {
      console.error("Failed to stop log monitor:", e);
      triggerPopup("모니터링 중지 실패", `로그 모니터링 중지에 실패했습니다: ${e}`, "error");
    }
  };

  const handleTestWindow = async () => {
    try {
      await invoke("test_abyssal_window");
      triggerPopup("테스트 완료", "어비셜 결과 창 테스트가 실행되었습니다.", "info");
    } catch (e) {
      console.error("Failed to test window:", e);
      triggerPopup("테스트 실패", `창 테스트에 실패했습니다: ${e}`, "error");
    }
  };



  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
    const { name, value } = e.target;
    setConfig(prevConfig => ({
      ...prevConfig,
      general: {
        ...prevConfig.general,
        [name]: value,
      },
    }));
    setIsDirty(true);
  };



  if (loading) {
    return (
      <div className="settings-loading">
        <div className="loading-spinner"></div>
        <p>⚙️ 설정을 불러오는 중...</p>
      </div>
    );
  }

  return (
    <div className="settings-panel">
      {/* Configuration Sections */}
      <div className="settings-content">
        {/* General Configuration */}
        <div className="config-section">
          <div className="section-header">
            <div className="section-icon">⚙️</div>
            <div className="section-info">
              <h2 className="section-title">🔧 일반 설정</h2>
              <p className="section-description">기본 애플리케이션 설정 및 환경 설정</p>
            </div>
          </div>
          
          <div className="config-grid">
            <div className="config-field">
              <label className="field-label" htmlFor="log_path">
                <span className="label-text">📁 로그 파일 경로</span>
                <span className="label-hint">EVE Online 로그 파일 위치</span>
              </label>
              <input
                type="text"
                id="log_path"
                name="log_path"
                value={config.general.log_path || ''}
                onChange={handleChange}
                placeholder="C:\Users\...\EVE\logs\Gamelogs"
                className="field-input"
              />
            </div>

            <div className="config-field">
              <label className="field-label" htmlFor="character_name">
                <span className="label-text">👤 캐릭터 이름</span>
                <span className="label-hint">EVE Online 캐릭터 이름</span>
              </label>
              <input
                type="text"
                id="character_name"
                name="character_name"
                value={config.general.character_name || ''}
                onChange={handleChange}
                placeholder="캐릭터 이름을 입력하세요"
                className="field-input"
              />
            </div>

            <div className="config-field">
              <label className="field-label">
                <span className="label-text">📊 CSV 데이터 폴더</span>
                <span className="label-hint">어비셜 결과 CSV 파일이 저장된 폴더</span>
              </label>
              <button
                onClick={handleOpenCsvFolder}
                className="control-button secondary"
              >
                <span className="button-icon">📂</span>
                <span className="button-text">CSV 폴더 열기</span>
              </button>
            </div>


          </div>
        </div>

        {/* Monitor Control */}
        <div className="config-section">
          <div className="section-header">
            <div className="section-icon">📡</div>
            <div className="section-info">
              <h2 className="section-title">📡 모니터링 제어</h2>
              <p className="section-description">로그 파일 모니터링 및 테스트 관리</p>
            </div>
          </div>

          <div className="control-grid">
            <div className="control-card">
              <div className="card-header">
                <div className="card-icon">🎯</div>
                <div className="card-title">로그 모니터</div>
              </div>
              <div className="card-content">
                <p className="card-description">
                  {logMonitorRunning 
                    ? '🟢 EVE 로그 파일에서 어비셜 런을 능동적으로 모니터링 중입니다' 
                    : '🔴 로그 모니터링이 현재 중지된 상태입니다'
                  }
                </p>
                <button
                  onClick={logMonitorRunning ? handleStopMonitor : handleStartMonitor}
                  className={`control-button ${logMonitorRunning ? 'danger' : 'primary'}`}
                >
                  <span className="button-icon">
                    {logMonitorRunning ? '⏹️' : '▶️'}
                  </span>
                  <span className="button-text">
                    {logMonitorRunning ? '모니터링 중지' : '모니터링 시작'}
                  </span>
                </button>
              </div>
            </div>

            <div className="control-card">
              <div className="card-header">
                <div className="card-icon">🧪</div>
                <div className="card-title">어비셜 결과 창 테스트</div>
              </div>
              <div className="card-content">
                <p className="card-description">
                  어비셜 결과 창이 정상적으로 표시되는지 테스트합니다
                </p>
                <button
                  onClick={handleTestWindow}
                  className="control-button secondary"
                >
                  <span className="button-icon">🚀</span>
                  <span className="button-text">테스트 실행</span>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div className="settings-footer">
        <div className="footer-content">
          <div className="footer-info">
            {isDirty && (
              <div className="unsaved-changes">
                <span className="changes-icon">●</span>
                <span className="changes-text">저장되지 않은 변경사항</span>
              </div>
            )}
          </div>
          <div className="footer-actions">
            <button
              onClick={() => loadConfig()}
              className="action-button secondary"
              disabled={loading || saving}
            >
              <span className="button-icon">🔄</span>
              <span className="button-text">초기화</span>
            </button>
            <button
              onClick={saveConfig}
              className="action-button primary"
              disabled={!isDirty || saving}
            >
              <span className="button-icon">{saving ? '⏳' : '💾'}</span>
              <span className="button-text">
                {saving ? '저장 중...' : '변경사항 저장'}
              </span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default Settings;