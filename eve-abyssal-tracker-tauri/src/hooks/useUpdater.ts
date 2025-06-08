import { useState, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';

export const useUpdater = (triggerPopup: (title: string, message: string, type?: "info" | "warning" | "error") => void) => {
  const [showUpdateDialog, setShowUpdateDialog] = useState(false);
  const [latestVersion, setLatestVersion] = useState("");
  const [currentVersion, setCurrentVersion] = useState("");
  const [isDownloadingUpdate, setIsDownloadingUpdate] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(0);
  const isCheckingForUpdate = useRef(false);

  const handleUpdate = useCallback(async () => {
    setIsDownloadingUpdate(true);
    setDownloadProgress(0);
    
    try {
      const progressInterval = setInterval(() => {
        setDownloadProgress(prev => {
          if (prev >= 95) return prev;
          return prev + Math.random() * 10;
        });
      }, 200);
      
      await invoke("download_and_install_update_command");
      
      clearInterval(progressInterval);
      setDownloadProgress(100);
      
      setShowUpdateDialog(false);
      
    } catch (error) {
      console.error("업데이트 실행 실패:", error);
      setIsDownloadingUpdate(false);
      setDownloadProgress(0);
      const errorMessage = error instanceof Error ? error.message : String(error);
      triggerPopup("업데이트 실패", `업데이트 중 오류가 발생했습니다: ${errorMessage}`, "error");
    }
  }, [triggerPopup]);

  const handleCloseUpdateDialog = useCallback(() => {
    setShowUpdateDialog(false);
    setIsDownloadingUpdate(false);
    setDownloadProgress(0);
  }, []);
  
  const checkForUpdates = useCallback(async (version: string) => {
    if (isCheckingForUpdate.current) {
        return;
    }
    isCheckingForUpdate.current = true;
    
    try {
      const updateInfo = await invoke("check_for_update_command") as { available: boolean, latest_version: string };
      if (updateInfo.available && updateInfo.latest_version !== version) {
        setLatestVersion(updateInfo.latest_version);
        setShowUpdateDialog(true);
      }
    } catch (error) {
        console.error("업데이트 확인 실패:", error);
        const errorMessage = error instanceof Error ? error.message : String(error);
        triggerPopup("업데이트 확인 실패", `최신 버전을 확인하는 중 오류가 발생했습니다: ${errorMessage}`, "error");
    } finally {
        isCheckingForUpdate.current = false;
    }
  }, [triggerPopup]);

  return {
    showUpdateDialog,
    latestVersion,
    currentVersion,
    isDownloadingUpdate,
    downloadProgress,
    handleUpdate,
    handleCloseUpdateDialog,
    checkForUpdates,
    setCurrentVersion,
    setLatestVersion,
    setShowUpdateDialog
  };
}; 