import { useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { RunData, AbyssalData, LoadingStep } from "../types";

export const useAbyssalData = (triggerPopup: (title: string, message: string, type?: "info" | "warning" | "error") => void) => {
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
      
      try {
        await invoke("reload_icon_cache");
        console.log('[INFO] IconCache reloaded after data analysis');
      } catch (cacheError) {
        console.warn('[WARN] Failed to reload icon cache:', cacheError);
      }
    } catch (err) {
      console.error("Failed to fetch abyssal data:", err);
      const errorMessage = err instanceof Error ? err.message : String(err);
      setDataError(`데이터 로딩 실패: ${errorMessage}`);
      triggerPopup("데이터 로딩 실패", `데이터를 불러오는 중 오류가 발생했습니다: ${errorMessage}`, "error");
      
      setLoadingSteps(prev => prev.map(step => ({ ...step, status: 'error' as const })));
    } finally {
      setDataLoading(false);
      isDataLoadingRef.current = false;
    }
  }, [triggerPopup, resetLoadingSteps]);

  const lightRefreshAbyssalData = useCallback(async () => {
    if (isDataLoadingRef.current) {
      return;
    }
    
    isDataLoadingRef.current = true;
    setDataError(null);
    
    try {
      const parsedResult = await invoke("light_refresh_abyssal_data_command") as AbyssalData;
      setAbyssalData(parsedResult);
      
      try {
        await invoke("reload_icon_cache");
        console.log('[INFO] IconCache reloaded after light refresh');
      } catch (cacheError) {
        console.warn('[WARN] Failed to reload icon cache after light refresh:', cacheError);
      }
    } catch (err) {
      console.error("Failed to light refresh abyssal data:", err);
      const errorMessage = err instanceof Error ? err.message : String(err);
      setDataError(`가벼운 데이터 새로고침 실패: ${errorMessage}`);
      triggerPopup("데이터 새로고침 실패", `데이터를 새로고침하는 중 오류가 발생했습니다: ${errorMessage}`, "error");
    } finally {
      isDataLoadingRef.current = false;
    }
  }, [triggerPopup]);

  const handleRunDeleted = useCallback((deletedRun: RunData) => {
    if (!abyssalData) return;

    const newData = { ...abyssalData };
    
    newData.df = newData.df.filter(run => 
      !(run['시작시각(KST)'] === deletedRun['시작시각(KST)'] && 
        run['종료시각(KST)'] === deletedRun['종료시각(KST)'])
    );
    
    const runDate = deletedRun['날짜'];
    if (newData.daily_stats[runDate]) {
      const filteredRuns = newData.daily_stats[runDate].runs.filter(run =>
        !(run['시작시각(KST)'] === deletedRun['시작시각(KST)'] && 
          run['종료시각(KST)'] === deletedRun['종료시각(KST)'])
      );
      
      if (filteredRuns.length === 0) {
        delete newData.daily_stats[runDate];
      } else {
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
    
    if (newData.df.length > 0) {
      newData.overall_stats.avg_isk = newData.df.reduce((sum, run) => sum + run['실수익'], 0) / newData.df.length;
      newData.overall_stats.avg_time = newData.df.reduce((sum, run) => sum + run['런 소요(분)'], 0) / newData.df.length;
      newData.overall_stats.avg_iskph = newData.df.reduce((sum, run) => sum + run['ISK/h'], 0) / newData.df.length;
      
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
          total_entry_cost: 0, // TODO: 실제 필라멘트 비용 계산 로직 필요
        };
      });
    } else {
        newData.overall_stats = {
            avg_isk: 0,
            avg_time: 0,
            avg_iskph: 0,
            tier_weather_stats: [],
        };
    }
    setAbyssalData(newData);
  }, [abyssalData]);

  const updateLoadingStep = useCallback((stepId: string, status: 'pending' | 'loading' | 'completed' | 'error', progress?: number, message?: string) => {
    setLoadingSteps(prevSteps => prevSteps.map(step =>
      step.id === stepId ? { ...step, status, progress, message: message ?? step.message } : step
    ));
  }, []);

  return {
    abyssalData,
    dataLoading,
    dataError,
    loadingSteps,
    setLoadingSteps,
    loadAbyssalData,
    lightRefreshAbyssalData,
    handleRunDeleted,
    updateLoadingStep
  };
}; 