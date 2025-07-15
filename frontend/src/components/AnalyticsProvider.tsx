'use client';

import { useEffect } from 'react';
import Analytics from '@/lib/analytics';

interface AnalyticsProviderProps {
  children: React.ReactNode;
}

export default function AnalyticsProvider({ children }: AnalyticsProviderProps) {
  useEffect(() => {
    const initAnalytics = async () => {
      // TODO: Replace with your actual PostHog API key
      const POSTHOG_API_KEY = process.env.NEXT_PUBLIC_POSTHOG_API_KEY || '';
      const ANALYTICS_ENABLED = process.env.NEXT_PUBLIC_ANALYTICS_ENABLED === 'true';
      
      if (POSTHOG_API_KEY && ANALYTICS_ENABLED) {
        await Analytics.init(POSTHOG_API_KEY, true);
        await Analytics.trackAppStarted();
        
        // Generate a unique user ID if not exists
        const userId = localStorage.getItem('meetily_user_id') || 
                      `user_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
        localStorage.setItem('meetily_user_id', userId);
        
        await Analytics.identify(userId, {
          app_version: '0.0.5',
          platform: 'tauri',
          first_seen: new Date().toISOString(),
        });
      } else {
        console.log('Analytics disabled or API key not provided');
      }
    };

    initAnalytics().catch(console.error);
  }, []);

  return <>{children}</>;
} 