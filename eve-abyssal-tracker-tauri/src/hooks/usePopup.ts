import { useState, useCallback } from 'react';

export const usePopup = () => {
  const [showPopup, setShowPopup] = useState(false);
  const [popupTitle, setPopupTitle] = useState("");
  const [popupMessage, setPopupMessage] = useState("");
  const [popupType, setPopupType] = useState<"info" | "warning" | "error">("info");

  const triggerPopup = useCallback((title: string, message: string, type: "info" | "warning" | "error" = "info") => {
    setPopupTitle(title);
    setPopupMessage(message);
    setPopupType(type);
    setShowPopup(true);
    setTimeout(() => {
      setShowPopup(false);
    }, 5000);
  }, []);

  const popupProps = {
    show: showPopup,
    title: popupTitle,
    message: popupMessage,
    type: popupType,
    onClose: () => setShowPopup(false)
  };

  return {
    triggerPopup,
    popupProps
  };
}; 