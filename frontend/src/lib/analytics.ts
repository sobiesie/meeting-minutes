import { invoke } from '@tauri-apps/api/core';

export interface AnalyticsProperties {
  [key: string]: string;
}

export class Analytics {
  private static initialized = false;

  static async init(apiKey: string, enabled: boolean = true): Promise<void> {
    try {
      await invoke('init_analytics', { apiKey, enabled });
      this.initialized = true;
      console.log('Analytics initialized successfully');
    } catch (error) {
      console.error('Failed to initialize analytics:', error);
    }
  }

  static async isEnabled(): Promise<boolean> {
    try {
      return await invoke('is_analytics_enabled');
    } catch (error) {
      console.error('Failed to check analytics status:', error);
      return false;
    }
  }

  static async track(eventName: string, properties?: AnalyticsProperties): Promise<void> {
    if (!this.initialized) {
      console.warn('Analytics not initialized');
      return;
    }

    try {
      await invoke('track_event', { eventName, properties });
    } catch (error) {
      console.error(`Failed to track event ${eventName}:`, error);
    }
  }

  static async identify(userId: string, properties?: AnalyticsProperties): Promise<void> {
    if (!this.initialized) {
      console.warn('Analytics not initialized');
      return;
    }

    try {
      await invoke('identify_user', { userId, properties });
    } catch (error) {
      console.error(`Failed to identify user ${userId}:`, error);
    }
  }

  // Meeting-specific tracking methods
  static async trackMeetingStarted(meetingId: string, meetingTitle: string): Promise<void> {
    if (!this.initialized) return;

    try {
      await invoke('track_meeting_started', { meetingId, meetingTitle });
    } catch (error) {
      console.error('Failed to track meeting started:', error);
    }
  }

  static async trackRecordingStarted(meetingId: string): Promise<void> {
    if (!this.initialized) return;

    try {
      await invoke('track_recording_started', { meetingId });
    } catch (error) {
      console.error('Failed to track recording started:', error);
    }
  }

  static async trackRecordingStopped(meetingId: string, durationSeconds?: number): Promise<void> {
    if (!this.initialized) return;

    try {
      await invoke('track_recording_stopped', { meetingId, durationSeconds });
    } catch (error) {
      console.error('Failed to track recording stopped:', error);
    }
  }

  static async trackMeetingDeleted(meetingId: string): Promise<void> {
    if (!this.initialized) return;

    try {
      await invoke('track_meeting_deleted', { meetingId });
    } catch (error) {
      console.error('Failed to track meeting deleted:', error);
    }
  }

  static async trackSearchPerformed(query: string, resultsCount: number): Promise<void> {
    if (!this.initialized) return;

    try {
      await invoke('track_search_performed', { query, resultsCount });
    } catch (error) {
      console.error('Failed to track search performed:', error);
    }
  }

  static async trackSettingsChanged(settingType: string, newValue: string): Promise<void> {
    if (!this.initialized) return;

    try {
      await invoke('track_settings_changed', { settingType, newValue });
    } catch (error) {
      console.error('Failed to track settings changed:', error);
    }
  }

  static async trackFeatureUsed(featureName: string): Promise<void> {
    if (!this.initialized) return;

    try {
      await invoke('track_feature_used', { featureName });
    } catch (error) {
      console.error('Failed to track feature used:', error);
    }
  }

  // Convenience methods for common events
  static async trackPageView(pageName: string): Promise<void> {
    await this.track('page_view', { page: pageName });
  }

  static async trackButtonClick(buttonName: string, location?: string): Promise<void> {
    const properties: AnalyticsProperties = { button: buttonName };
    if (location) properties.location = location;
    await this.track('button_click', properties);
  }

  static async trackError(errorType: string, errorMessage: string): Promise<void> {
    await this.track('error', { 
      error_type: errorType, 
      error_message: errorMessage 
    });
  }

  static async trackAppStarted(): Promise<void> {
    await this.track('app_started', { 
      timestamp: new Date().toISOString() 
    });
  }
}

export default Analytics; 