import React, { useState, useEffect, useCallback } from 'react';

import './Settings.css';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { invoke } from "@tauri-apps/api/core";
import { LocationInfo } from '../types';

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
  onSettingsSaved: () => void;
  triggerPopup: (title: string, message: string, type?: "info" | "warning" | "error") => void;
}

const Settings: React.FC<SettingsProps> = ({ onSettingsSaved, triggerPopup }) => {
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
  const [abyssalWindowEnabled, setAbyssalWindowEnabled] = useState(true);
  const [locationInfo, setLocationInfo] = useState<LocationInfo>({
    current_system: null,
    previous_system: null,
    last_updated: null,
  });
  const [logFileInfo, setLogFileInfo] = useState<{
    file_name: string;
    full_path: string;
    file_size: number;
    modified_time: string;
    monitoring: boolean;
  } | null>(null);


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

  const loadLocationInfo = useCallback(async () => {
    try {
      const result = await invoke("get_location_info_command") as LocationInfo;
      setLocationInfo(result);
    } catch (e) {
      console.error("Failed to load location info:", e);
      // 위치 정보 로딩 실패는 크리티컬하지 않으므로 조용히 처리
    }
  }, []);

  const loadAbyssalWindowState = useCallback(async () => {
    try {
      const enabled = await invoke("get_abyssal_window_enabled") as boolean;
      setAbyssalWindowEnabled(enabled);
    } catch (e) {
      console.error("Failed to load abyssal window state:", e);
      // 실패 시 기본값 유지
    }
  }, []);

  const loadLogFileInfo = useCallback(async () => {
    try {
      const result = await invoke("get_current_log_file_info") as { file_name: string; full_path: string; file_size: number; modified_time: string; monitoring: boolean } | null;
      setLogFileInfo(result);
    } catch (e) {
      console.error("Failed to load log file info:", e);
      // 로그 파일 정보 로딩 실패는 조용히 처리
    }
  }, []);

  const saveConfig = async () => {
    setSaving(true);
    try {
      await invoke("set_log_path", { path: config.general.log_path });
      await invoke("set_character_name", { characterName: config.general.character_name });
      triggerPopup("설정 저장 완료", "설정이 성공적으로 저장되었습니다.", "info");
      setIsDirty(false);
      onSettingsSaved(); // Notify parent that settings were saved
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

  const handleToggleAbyssalWindow = async () => {
    const newState = !abyssalWindowEnabled;
    try {
      await invoke("set_abyssal_window_enabled", { enabled: newState });
      setAbyssalWindowEnabled(newState);
      triggerPopup(
        newState ? "어비셜 창 활성화" : "어비셜 창 비활성화", 
        newState 
          ? "어비셜 런 완료 시 결과 창이 자동으로 표시됩니다." 
          : "어비셜 런 완료 시 결과 창이 더 이상 표시되지 않습니다.", 
        "info"
      );
    } catch (e) {
      console.error("Failed to toggle abyssal window:", e);
      triggerPopup("설정 변경 실패", `어비셜 창 설정 변경에 실패했습니다: ${e}`, "error");
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
    loadLocationInfo();
    loadAbyssalWindowState();
    loadLogFileInfo();
    
    // 로그 모니터링은 항상 실행되므로 주기적으로 위치 정보와 로그 파일 정보 업데이트
    const intervalId = setInterval(() => {
      loadLocationInfo();
      loadLogFileInfo();
    }, 5000); // 5초마다 업데이트
    
    return () => {
      clearInterval(intervalId);
    };
  }, [loadConfig, loadLocationInfo, loadAbyssalWindowState, loadLogFileInfo]);

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

  const formatLastUpdated = (lastUpdated: string | null): string => {
    if (!lastUpdated) return '정보 없음';
    try {
      const date = new Date(lastUpdated);
      // 한국시간(KST = UTC+9)으로 변환
      const kstDate = new Date(date.getTime() + (9 * 60 * 60 * 1000));
      return kstDate.toLocaleString('ko-KR', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
        hour12: false,
        timeZone: 'Asia/Seoul'
      });
    } catch {
      return '정보 없음';
    }
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
              <h2 className="section-title">일반 설정</h2>
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
              <h2 className="section-title">모니터링 제어</h2>
              <p className="section-description">로그 파일은 항상 모니터링되며, 어비셜 결과 창 표시를 제어할 수 있습니다</p>
            </div>
          </div>

          <div className="control-grid">
            <div className="control-card">
              <div className="card-header">
                <div className="card-icon">🎯</div>
                <div className="card-title">어비셜 결과 창</div>
              </div>
              <div className="card-content">
                <p className="card-description">
                  {abyssalWindowEnabled 
                    ? '🟢 어비셜 런 완료 시 결과 창이 자동으로 표시됩니다' 
                    : '🔴 어비셜 런 완료 시 결과 창이 표시되지 않습니다'
                  }
                </p>
                <button
                  onClick={handleToggleAbyssalWindow}
                  className={`control-button ${abyssalWindowEnabled ? 'danger' : 'primary'}`}
                >
                  <span className="button-icon">
                    {abyssalWindowEnabled ? '🔇' : '🔊'}
                  </span>
                  <span className="button-text">
                    {abyssalWindowEnabled ? '결과 창 비활성화' : '결과 창 활성화'}
                  </span>
                </button>
                <p className="card-description">
                  🧪 테스트 실행: 어비셜 결과 창이 정상적으로 표시되는지 확인할 수 있습니다
                </p>
                <button
                  onClick={handleTestWindow}
                  className="control-button secondary"
                >
                  <span className="button-icon">🧪</span>
                  <span className="button-text">테스트 실행</span>
                </button>
              </div>
            </div>

            <div className="control-card">
              <div className="card-header">
                <div className="card-icon">📍</div>
                <div className="card-title">현재 위치 정보</div>
              </div>
              <div className="card-content">
                <div className="location-info">
                  <div className="location-item">
                    <span className="location-label">🎯 현재 성계:</span>
                    <span className="location-value">
                      {locationInfo.current_system || '정보 없음'}
                    </span>
                  </div>
                  <div className="location-item">
                    <span className="location-label">↩️ 이전 성계:</span>
                    <span className="location-value">
                      {locationInfo.previous_system || '정보 없음'}
                    </span>
                  </div>
                  <div className="location-item">
                    <span className="location-label">🕐 마지막 업데이트:</span>
                    <span className="location-value location-time">
                      {formatLastUpdated(locationInfo.last_updated)}
                    </span>
                  </div>
                </div>
                <button
                  onClick={loadLocationInfo}
                  className="control-button secondary small"
                >
                  <span className="button-icon">🔄</span>
                  <span className="button-text">새로고침</span>
                </button>
              </div>
            </div>

            <div className="control-card">
              <div className="card-header">
                <div className="card-icon">📄</div>
                <div className="card-title">모니터링 중인 로그 파일</div>
              </div>
              <div className="card-content">
                {logFileInfo ? (
                  <div className="location-info">
                    <div className="location-item">
                      <span className="location-label">📝 파일 이름</span>
                      <span 
                        className="location-value log-file-name"
                        title={logFileInfo?.full_path || '로그 파일을 찾을 수 없습니다.'}
                      >
                        {logFileInfo?.file_name || '로그 파일을 찾을 수 없습니다.'}
                      </span>
                    </div>
                    <div className="location-item">
                      <span className="location-label">🔄 마지막 수정</span>
                      <span className="location-value">{logFileInfo?.modified_time || '정보 없음'}</span>
                    </div>
                  </div>
                ) : (
                  <p className="card-description">
                    🔍 현재 모니터링 중인 로그 파일이 없습니다
                  </p>
                )}
                <button
                  onClick={loadLogFileInfo}
                  className="control-button secondary small"
                >
                  <span className="button-icon">🔄</span>
                  <span className="button-text">새로고침</span>
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