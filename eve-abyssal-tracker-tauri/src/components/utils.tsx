import React from 'react';
import { invoke } from '@tauri-apps/api/core';

export const parseItems = (itemString: string): [string, number][] => {
  if (!itemString) return [];
  const items = itemString.split('; ').map(item => item.trim());
  const parsed: [string, number][] = [];
  items.forEach(item => {
    const match = item.match(/^(.*?)(?:\s+(\d+))?$/);
    if (match) {
      const itemName = match[1].trim();
      const quantity = match[2] ? parseInt(match[2]) : 1;
      parsed.push([itemName, quantity]);
    } else {
      parsed.push([item, 1]); // 수량 명시 없으면 1개로 처리
    }
  });
  return parsed;
};

export const aggregateItems = (parsedItems: [string, number][], itemBuyPriceCache: { [key: string]: number }) => {
  const aggregated: { [key: string]: { '아이템 이름': string; '개수': number; '개당 가격': number; '총 가격': number; '총 가격_num': number } } = {};
  parsedItems.forEach(([itemName, itemQty]) => {
    const price = Math.round(itemBuyPriceCache[itemName] || itemBuyPriceCache[itemName.replace('*', '').trim()] || 0);
    const totalPrice = Math.round(price * itemQty);
    if (aggregated[itemName]) {
      aggregated[itemName]['개수'] += itemQty;
      aggregated[itemName]['총 가격'] += totalPrice;
      aggregated[itemName]['총 가격_num'] += totalPrice;
    } else {
      aggregated[itemName] = {
        '아이템 이름': itemName,
        '개수': itemQty,
        '개당 가격': price,
        '총 가격': totalPrice,
        '총 가격_num': totalPrice
      };
    }
  });
  return Object.values(aggregated).sort((a, b) => b['총 가격_num'] - a['총 가격_num']);
};

// 아이콘 관련 유틸리티 함수들
export const getItemTypeId = async (itemName: string): Promise<number | null> => {
  try {
    const result = await invoke<number | null>('get_type_id', { itemName });
    return result;
  } catch (error) {
    console.error('Failed to get type ID:', error);
    return null;
  }
};

// 필라멘트 이름 가져오기
export const getFilamentName = async (abyssalType: string): Promise<string | null> => {
  try {
    const result = await invoke<string | null>('get_filament_name', { abyssalType });
    return result;
  } catch (error) {
    console.error('Failed to get filament name:', error);
    return null;
  }
};

// 여러 아이템의 type_id를 가져오는 함수는 제거 (필요시 개별 호출)

export const getIconUrl = async (typeId: number): Promise<string> => {
  try {
    return await invoke('get_icon_url', { typeId });
  } catch (error) {
    console.error('Failed to get icon URL:', error);
    return `https://images.evetech.net/types/${typeId}/icon`;
  }
};

export const getBestImageUrl = async (typeId: number, itemName: string): Promise<string> => {
  try {
    return await invoke('get_best_image_url', { typeId, itemName });
  } catch (error) {
    console.error('Failed to get best image URL:', error);
    // 실패 시 기본 아이콘 URL로 폴백
    return `https://images.evetech.net/types/${typeId}/icon`;
  }
};

// 런 타입 배지 컴포넌트
interface RunTypeBadgeProps {
  abyssalType: string;
  className?: string;
}

export const RunTypeBadge: React.FC<RunTypeBadgeProps> = ({ abyssalType, className = '' }) => {
  const [filamentIcon, setFilamentIcon] = React.useState<string>('');
  const [loading, setLoading] = React.useState(true);

  React.useEffect(() => {
    const loadFilamentIcon = async () => {
      try {
        setLoading(true);
        
        // 어비셜 타입에서 필라멘트 이름 가져오기
        const filamentName = await getFilamentName(abyssalType);
        if (!filamentName) {
          setLoading(false);
          return;
        }
        
        // 필라멘트의 타입 ID 가져오기
        const typeId = await getItemTypeId(filamentName);
        if (!typeId) {
          setLoading(false);
          return;
        }
        
        // 최적의 이미지 URL 가져오기
        const imageUrl = await getBestImageUrl(typeId, filamentName);
        setFilamentIcon(imageUrl);
      } catch (err) {
        console.error(`Failed to load filament icon for ${abyssalType}:`, err);
      } finally {
        setLoading(false);
      }
    };

    loadFilamentIcon();
  }, [abyssalType]);

  return (
    <div className={`run-type-badge ${className}`} style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
      {!loading && filamentIcon && (
        <img
          src={filamentIcon}
          alt={abyssalType}
          style={{ width: 16, height: 16, borderRadius: '2px', flexShrink: 0 }}
        />
      )}
      {abyssalType}
    </div>
  );
};

// 아이템 아이콘 컴포넌트
interface ItemIconProps {
  typeId?: number;
  itemName: string;
  size?: number;
  className?: string;
}

export const ItemIcon: React.FC<ItemIconProps> = ({ typeId, itemName, size = 32, className = '' }) => {
  const [iconSrc, setIconSrc] = React.useState<string>('');
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState(false);
  const [retryCount, setRetryCount] = React.useState(0);
  const [actualTypeId, setActualTypeId] = React.useState<number | null>(null);

  React.useEffect(() => {
    const loadIcon = async () => {
      try {
        setLoading(true);
        setError(false);
        
        let resolvedTypeId = typeId;
        
        // typeId가 없으면 itemName으로 조회
        if (!resolvedTypeId) {
          // * 제거하고 깨끗한 아이템 이름으로 조회
          const cleanItemName = itemName.replace('*', '').trim();
          const fetchedTypeId = await getItemTypeId(cleanItemName);
          if (!fetchedTypeId) {
            setError(true);
            setLoading(false);
            return;
          }
          resolvedTypeId = fetchedTypeId;
        }
        
        setActualTypeId(resolvedTypeId);
        
        // 최적의 이미지 URL 가져오기 (icon -> bp 폴백 로직이 백엔드에 구현됨)
        const imageUrl = await getBestImageUrl(resolvedTypeId, itemName);
        setIconSrc(imageUrl);
      } catch (err) {
        console.error(`Failed to load icon for ${itemName}:`, err);
        setError(true);
      } finally {
        setLoading(false);
      }
    };

    loadIcon();
  }, [typeId, itemName, retryCount]);

  const handleImageError = React.useCallback(() => {
    if (retryCount < 1 && actualTypeId) {
      // 한 번만 bp로 재시도
      const bpUrl = `https://images.evetech.net/types/${actualTypeId}/bp`;
      setIconSrc(bpUrl);
      setRetryCount(1);
    } else {
      setError(true);
    }
  }, [actualTypeId, retryCount]);

  if (loading) {
    return (
      <div 
        className={`item-icon-placeholder loading ${className}`}
        style={{ 
          width: size, 
          height: size, 
          display: 'flex', 
          alignItems: 'center', 
          justifyContent: 'center', 
          backgroundColor: 'var(--tertiary-bg)', 
          borderRadius: '4px',
          border: '1px solid var(--border-primary)',
          color: 'var(--text-accent)'
        }}
      >
        ⏳
      </div>
    );
  }

  if (error || !iconSrc) {
    return (
      <div 
        className={`item-icon-placeholder error ${className}`}
        style={{ 
          width: size, 
          height: size, 
          display: 'flex', 
          alignItems: 'center', 
          justifyContent: 'center', 
          backgroundColor: 'var(--secondary-bg)', 
          borderRadius: '4px',
          border: '1px solid var(--border-primary)',
          color: 'var(--text-muted)'
        }}
        title={itemName}
      >
        📦
      </div>
    );
  }

  return (
    <img
      src={iconSrc}
      alt={itemName}
      title={itemName}
      className={`item-icon ${className}`}
      style={{ width: size, height: size, borderRadius: '4px' }}
      onError={handleImageError}
    />
  );
};

export const tableHeaderStyle: React.CSSProperties = {
  padding: '10px',
  borderBottom: '1px solid #ddd',
  textAlign: 'left',
  backgroundColor: '#f8f8f8'
};

export const tableCellStyle: React.CSSProperties = {
  padding: '10px',
  borderBottom: '1px solid #eee',
  textAlign: 'left'
};