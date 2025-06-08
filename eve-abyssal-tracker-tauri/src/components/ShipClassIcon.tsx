import React, { useState } from 'react';

interface ShipClassIconProps {
  shipClass: number;
  size?: number;
  showTooltip?: boolean;
  className?: string;
}

const ShipClassIcon: React.FC<ShipClassIconProps> = ({ 
  shipClass, 
  size = 16, 
  showTooltip = true,
  className = ''
}) => {
  const [imageError, setImageError] = useState(false);
  
  // 함급별 이모지 fallback
  const getFallbackIcon = (shipClass: number): string => {
    switch (shipClass) {
      case 3: return '🚀'; // 프리깃
      case 2: return '⚡'; // 디스트로이어
      case 1: return '🛡️'; // 크루저
      default: return '🚢';
    }
  };

  const getDisplayName = (shipClass: number): string => {
    switch (shipClass) {
      case 3: return '프리깃';
      case 2: return '디스트로이어';
      case 1: return '크루저';
      default: return `등급 ${shipClass}`;
    }
  };

  const displayName = getDisplayName(shipClass);

  // PNG 파일이 있을 때는 이미지, 없을 때는 이모지 fallback 사용
  if (imageError) {
    return (
      <span
        title={showTooltip ? displayName : undefined}
        className={`ship-class-icon-fallback ${className}`}
        style={{
          fontSize: size - 2,
          lineHeight: 1,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center'
        }}
      >
        {getFallbackIcon(shipClass)}
      </span>
    );
  }

  const getImagePath = (shipClass: number): string => {
    switch (shipClass) {
      case 3: return '/icons/frigate.png';
      case 2: return '/icons/destroyer.png';
      case 1: return '/icons/cruiser.png';
      default: return '/icons/cruiser.png';
    }
  };

  return (
    <img
      src={getImagePath(shipClass)}
      alt={displayName}
      title={showTooltip ? displayName : undefined}
      className={`ship-class-icon ${className}`}
      style={{
        width: size,
        height: size,
        borderRadius: '2px',
        flexShrink: 0,
        objectFit: 'contain'
      }}
      onError={() => setImageError(true)}
    />
  );
};

export default ShipClassIcon; 