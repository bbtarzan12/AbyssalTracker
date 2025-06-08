import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { LoadingProgressEvent } from '../types';

type AppInitProps = {
    updateLoadingStep: (stepId: string, status: 'pending' | 'loading' | 'completed' | 'error', progress?: number, message?: string) => void;
    loadAbyssalData: () => Promise<void>;
    checkForUpdates: (version: string) => Promise<void>;
    setCurrentVersion: (version: string) => void;
    triggerPopup: (title: string, message: string, type?: "info" | "warning" | "error") => void;
};

export const useAppInit = ({
    updateLoadingStep,
    loadAbyssalData,
    checkForUpdates,
    setCurrentVersion,
    triggerPopup,
}: AppInitProps) => {
    const [appInitializing, setAppInitializing] = useState(true);
    const [needsInitialSetup, setNeedsInitialSetup] = useState(false);
    const [checkingConfig, setCheckingConfig] = useState(true);

    const initializeApp = useCallback(async () => {
        setCheckingConfig(true);
        try {
            const config = await invoke("get_config") as { general: { log_path: string, character_name: string }};
            const configValid = config.general && config.general.log_path && config.general.character_name;

            if (configValid) {
                setNeedsInitialSetup(false);
                await loadAbyssalData();
                const version = await invoke("get_current_version") as string;
                setCurrentVersion(version);
                await checkForUpdates(version);
            } else {
                setNeedsInitialSetup(true);
            }
        } catch (error) {
            console.error("초기 설정 확인 실패:", error);
            const errorMessage = error instanceof Error ? error.message : String(error);
            triggerPopup("설정 오류", `설정 확인 중 오류가 발생했습니다: ${errorMessage}`, "error");
            setNeedsInitialSetup(true);
        } finally {
            setCheckingConfig(false);
            setAppInitializing(false);
        }
    }, [loadAbyssalData, checkForUpdates, setCurrentVersion, triggerPopup]);

    useEffect(() => {
        initializeApp();
    }, [initializeApp]);

    useEffect(() => {
        const unlisten = listen<LoadingProgressEvent>("loading_progress", (event) => {
            const { step, message, progress, completed } = event.payload;
            updateLoadingStep(step, completed ? 'completed' : 'loading', progress, message);
        });

        return () => {
            unlisten.then(f => f());
        };
    }, [updateLoadingStep]);

    const handleInitialSetupComplete = () => {
        setNeedsInitialSetup(false);
        setAppInitializing(true); // Re-initialize to load data
        initializeApp();
    };
    
    return {
        appInitializing,
        needsInitialSetup,
        checkingConfig,
        handleInitialSetupComplete
    };
}; 