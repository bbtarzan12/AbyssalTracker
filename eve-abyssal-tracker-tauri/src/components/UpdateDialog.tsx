import React from 'react';
import './UpdateDialog.css';

interface UpdateDialogProps {
  latestVersion: string;
  currentVersion: string;
  onClose: () => void;
  onUpdate: () => void;
  isDownloading?: boolean;
  downloadProgress?: number;
}

const UpdateDialog: React.FC<UpdateDialogProps> = ({ 
  latestVersion, 
  currentVersion, 
  onClose, 
  onUpdate,
  isDownloading = false,
  downloadProgress = 0
}) => {
  return (
    <div className="update-dialog-overlay">
      <div className="update-dialog">
        <div className="update-dialog-header">
          <h2>🔄 업데이트 알림</h2>
        </div>
        <div className="update-dialog-content">
          {!isDownloading ? (
            <>
              <p>새로운 버전이 출시되었습니다!</p>
              <div className="version-info">
                <div className="version-row">
                  <strong>현재 버전:</strong> v{currentVersion}
                </div>
                <div className="version-row">
                  <strong>최신 버전:</strong> v{latestVersion}
                </div>
              </div>
              <p>지금 업데이트하시겠습니까?</p>
            </>
          ) : (
            <>
              <p>업데이트를 다운로드하고 있습니다...</p>
              <div className="download-progress">
                <div className="progress-bar">
                  <div 
                    className="progress-fill" 
                    style={{ width: `${downloadProgress}%` }}
                  ></div>
                </div>
                <div className="progress-text">
                  {downloadProgress.toFixed(1)}%
                </div>
              </div>
              <p>잠시만 기다려주세요.</p>
            </>
          )}
        </div>
        <div className="update-dialog-actions">
          {!isDownloading ? (
            <>
              <button 
                className="update-dialog-btn secondary" 
                onClick={onClose}
              >
                나중에
              </button>
              <button 
                className="update-dialog-btn primary" 
                onClick={onUpdate}
              >
                업데이트
              </button>
            </>
          ) : (
            <button 
              className="update-dialog-btn secondary" 
              onClick={onClose}
            >
              취소
            </button>
          )}
        </div>
      </div>
    </div>
  );
};

export default UpdateDialog; 