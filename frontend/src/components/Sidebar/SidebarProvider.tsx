'use client';

import React, { createContext, useContext, useState, useEffect } from 'react';
import { usePathname, useRouter } from 'next/navigation';
import { load } from '@tauri-apps/plugin-store';


interface SidebarItem {
  id: string;
  title: string;
  type: 'folder' | 'file';
  children?: SidebarItem[];
}

export interface CurrentMeeting {
  id: string;
  title: string;
}

// Search result type for transcript search
interface TranscriptSearchResult {
  id: string;
  title: string;
  matchContext: string;
  timestamp: string;
};

interface SidebarContextType {
  currentMeeting: CurrentMeeting | null;
  setCurrentMeeting: (meeting: CurrentMeeting | null) => void;
  sidebarItems: SidebarItem[];
  isCollapsed: boolean;
  toggleCollapse: () => void;
  meetings: CurrentMeeting[];
  setMeetings: (meetings: CurrentMeeting[]) => void;
  isMeetingActive: boolean;
  setIsMeetingActive: (active: boolean) => void;
  isRecording: boolean;
  setIsRecording: (recording: boolean) => void;
  handleRecordingToggle: () => void;
  searchTranscripts: (query: string) => Promise<void>;
  searchResults: TranscriptSearchResult[];
  isSearching: boolean;
  setServerAddress: (address: string) => void;
  serverAddress: string;
  transcriptServerAddress: string;
  setTranscriptServerAddress: (address: string) => void;
}

const SidebarContext = createContext<SidebarContextType | null>(null);

export const useSidebar = () => {
  const context = useContext(SidebarContext);
  if (!context) {
    throw new Error('useSidebar must be used within a SidebarProvider');
  }
  return context;
};

export function SidebarProvider({ children }: { children: React.ReactNode }) {
  const [currentMeeting, setCurrentMeeting] = useState<CurrentMeeting | null>({ id: 'intro-call', title: '+ New Call' });
  const [isCollapsed, setIsCollapsed] = useState(true);
  const [meetings, setMeetings] = useState<CurrentMeeting[]>([]);
  const [sidebarItems, setSidebarItems] = useState<SidebarItem[]>([]);
  const [isMeetingActive, setIsMeetingActive] = useState(false);
  const [isRecording, setIsRecording] = useState(false);
  const [searchResults, setSearchResults] = useState<any[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [serverAddress, setServerAddress] = useState('');
  const [transcriptServerAddress, setTranscriptServerAddress] = useState('');


  const pathname = usePathname();
  const router = useRouter();

  useEffect(() => {
    const fetchMeetings = async () => {
      try {
        const store = await load('store.json', { autoSave: false });
        let serverAddress = await store.get('appServerUrl') as string | null;
        let transcriptServerAddress = await store.get('transcriptServerUrl') as string | null;
        if (!serverAddress) {
          await store.set('appServerUrl', 'http://localhost:5167');
          serverAddress = await store.get('appServerUrl') as string;
          await store.save();
        }
        if (!transcriptServerAddress) {
          await store.set('transcriptServerUrl', 'http://127.0.0.1:8178/stream');
          transcriptServerAddress = await store.get('transcriptServerUrl') as string;
          await store.save();
        }
        setServerAddress(serverAddress);
        setTranscriptServerAddress(transcriptServerAddress);
        const response = await fetch(`${serverAddress}/get-meetings`, {
          cache: 'no-store',
          headers: {
            'Cache-Control': 'no-cache, no-store, must-revalidate',
            'Pragma': 'no-cache',
            'Expires': '0'
          }
        });
        const data = await response.json();
        // Transform the response into the expected format
        const transformedMeetings = data.map((meeting: any) => ({
          id: meeting.id,
          title: meeting.title
        }));
        setMeetings(transformedMeetings);
        router.push('/')
      } catch (error) {
        console.error('Error fetching meetings:', error);
        setMeetings([]);
      }
    };
    fetchMeetings();
  }, []);

  const baseItems: SidebarItem[] = [
    ...meetings.map(meeting => ({ id: meeting.id, title: meeting.title, type: 'file' as const })),
    // {
    //   id: 'meetings',
    //   title: 'Meetings',
    //   type: 'folder' as const,
    //   children: [
    //     ...meetings.map(meeting => ({ id: meeting.id, title: meeting.title, type: 'file' as const }))
    //   ]
    // },
  ];

 

  const toggleCollapse = () => {
    setIsCollapsed(!isCollapsed);
  };

  // Update current meeting when on home page
  useEffect(() => {
    if (pathname === '/') {
      setCurrentMeeting({ id: 'intro-call', title: '+ New Call' });
    }
    setSidebarItems(baseItems);
  }, [pathname]);

  // Update sidebar items when meetings change
  useEffect(() => {
    setSidebarItems(baseItems);
  }, [meetings]);

  // Function to handle recording toggle from sidebar
  const handleRecordingToggle = () => {
    if (!isRecording) {
      // If not recording, navigate to home page and set flag to start recording automatically
      sessionStorage.setItem('autoStartRecording', 'true');
      router.push('/');
    }
    // The actual recording start/stop is handled in the Home component
  };
  
  // Function to search through meeting transcripts
  const searchTranscripts = async (query: string) => {
    if (!query.trim()) {
      setSearchResults([]);
      return;
    }
    
    try {
      setIsSearching(true);
      const response = await fetch(`${serverAddress}/search-transcripts`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ query }),
      });
      
      if (!response.ok) {
        throw new Error('Failed to search transcripts');
      }
      
      const results = await response.json();
      setSearchResults(results);
    } catch (error) {
      console.error('Error searching transcripts:', error);
      setSearchResults([]);
    } finally {
      setIsSearching(false);
    }
  };

  return (
    <SidebarContext.Provider value={{ 
      currentMeeting, 
      setCurrentMeeting, 
      sidebarItems, 
      isCollapsed, 
      toggleCollapse, 
      meetings, 
      setMeetings, 
      isMeetingActive, 
      setIsMeetingActive,
      isRecording,
      setIsRecording,
      handleRecordingToggle,
      searchTranscripts,
      searchResults,
      isSearching,
      setServerAddress,
      serverAddress,
      transcriptServerAddress,
      setTranscriptServerAddress
    }}>
      {children}
    </SidebarContext.Provider>
  );
}
