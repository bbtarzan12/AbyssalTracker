import React from 'react';
import './LoadingProgress.css';
import TitleBar from './TitleBar';

interface LoadingStep {
  id: string;
  name: string;
  status: 'pending' | 'loading' | 'completed' | 'error';
  progress?: number;
  message?: string;
}

interface LoadingProgressProps {
  show: boolean;
  steps: LoadingStep[];
  title?: string;
}

const LoadingProgress: React.FC<LoadingProgressProps> = ({ 
  show, 
  steps, 
  title = "🔄 데이터 로딩 및 분석" 
}) => {
  if (!show) return null;

  const totalSteps = steps.length;
  const completedSteps = steps.filter(step => step.status === 'completed').length;
  const currentStep = steps.find(step => step.status === 'loading');
  const overallProgress = totalSteps > 0 ? (completedSteps / totalSteps) * 100 : 0;

  const getStatusIcon = (status: LoadingStep['status']) => {
    switch (status) {
      case 'completed':
        return '✓';
      case 'loading':
        return '◐';
      case 'error':
        return '✗';
      default:
        return '○';
    }
  };

  const formatStepName = (name: string) => {
    // 이미 한국어로 되어있는 이름들을 그대로 사용
    return name;
  };

  return (
    <>
      <TitleBar />
      <div className="loading-overlay">
        <div className="loading-modal">
        {/* Compact Header */}
        <div className="loading-header">
          <div className="header-row">
            <div className="loading-brand">
              <div className="brand-icon">⚡</div>
              <div className="brand-text">EVE 어비셜 트래커</div>
            </div>
            <div className="progress-stats">
              <span className="progress-text">{completedSteps}/{totalSteps}</span>
              <span className="progress-percentage">{Math.round(overallProgress)}%</span>
            </div>
          </div>
          <div className="loading-title">{title}</div>
          <div className="progress-container">
            <div className="progress-track">
              <div 
                className="progress-fill" 
                style={{ width: `${overallProgress}%` }}
              />
            </div>
          </div>
        </div>

        {/* Current Step Display */}
        {currentStep && (
          <div className="current-step">
            <div className="current-step-icon">
              <div className="step-icon loading">{getStatusIcon('loading')}</div>
            </div>
            <div className="current-step-info">
              <div className="current-step-name">{formatStepName(currentStep.name)}</div>
              {currentStep.message && (
                <div className="current-step-message">{currentStep.message}</div>
              )}
            </div>
          </div>
        )}

        {/* Compact Steps List */}
        <div className="steps-list">
          {steps.map((step, _) => (
            <div key={step.id} className={`step-item ${step.status}`}>
              <div className="step-marker">
                <div className="marker-icon">{getStatusIcon(step.status)}</div>
              </div>
              <div className="step-content">
                <div className="step-name">{formatStepName(step.name)}</div>
                {step.status === 'loading' && step.progress !== undefined && (
                  <div className="step-progress">
                    <div className="step-progress-track">
                      <div 
                        className="step-progress-fill"
                        style={{ width: `${step.progress}%` }}
                      />
                    </div>
                    <span className="step-progress-text">{Math.round(step.progress)}%</span>
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
    </>
  );
};

export default LoadingProgress; 