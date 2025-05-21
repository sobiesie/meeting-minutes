'use client';

import React from 'react';

interface LoadingAnimationProps {
  isInitializing: boolean;
}

export const LoadingAnimation: React.FC<LoadingAnimationProps> = ({ isInitializing }) => {
  if (!isInitializing) return null;

  return (
    <div className="absolute inset-0 flex items-center justify-center bg-white bg-opacity-75 rounded-full">
      <div className="relative">
        <div className="w-8 h-8 border-4 border-red-200 rounded-full"></div>
        <div className="absolute top-0 left-0 w-8 h-8 border-4 border-red-500 rounded-full animate-spin border-t-transparent"></div>
      </div>
    </div>
  );
}; 