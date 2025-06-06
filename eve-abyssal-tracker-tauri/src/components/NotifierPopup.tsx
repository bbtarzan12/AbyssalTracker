import React from 'react';
import { createPortal } from 'react-dom';
import './NotifierPopup.css';

interface NotifierPopupProps {
  show: boolean;
  title: string;
  message: string;
  type: "info" | "warning" | "error";
  onClose: () => void;
}

const NotifierPopup: React.FC<NotifierPopupProps> = ({ show, title, message, type, onClose }) => {
  if (!show) {
    return null;
  }

  const getIcon = () => {
    switch (type) {
      case 'info':
        return '💡';
      case 'warning':
        return '⚠️';
      case 'error':
        return '❌';
      default:
        return 'ℹ️';
    }
  };
  
  return createPortal(
    <div className={`notifier-popup ${type}`}>
      <div className="notifier-content">
        <div className="notifier-header">
          <div className="notifier-icon">
            {getIcon()}
          </div>
          <h3 className="notifier-title">{title}</h3>
          <button 
            className="notifier-close"
            onClick={onClose}
            aria-label="닫기"
          >
            ✕
          </button>
        </div>
        <p className="notifier-message">{message}</p>
      </div>
      <div className="notifier-progress"></div>
    </div>,
    document.body
  );
};

export default NotifierPopup;