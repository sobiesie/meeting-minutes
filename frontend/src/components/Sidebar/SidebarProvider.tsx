'use client';

import React, { createContext, useContext, useState, useEffect } from 'react';
import { usePathname } from 'next/navigation';

interface SidebarItem {
  id: string;
  title: string;
  type: 'folder' | 'file';
  children?: SidebarItem[];
}

interface CurrentMeeting {
  id: string;
  title: string;
}

interface SidebarContextType {
  currentMeeting: CurrentMeeting | null;
  setCurrentMeeting: (meeting: CurrentMeeting | null) => void;
  sidebarItems: SidebarItem[];
  isCollapsed: boolean;
  toggleCollapse: () => void;
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
  const [currentMeeting, setCurrentMeeting] = useState<CurrentMeeting | null>(null);
  const [isCollapsed, setIsCollapsed] = useState(true);
  const [meetings, setMeetings] = useState<CurrentMeeting[]>([]);
  const pathname = usePathname();

  useEffect(() => {
    const fetchMeetings = async () => {
      try {
        const response = await fetch('http://localhost:5167/get-meetings', {
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
      } catch (error) {
        console.error('Error fetching meetings:', error);
        setMeetings([]);
      }
    };
    fetchMeetings();
  }, []);

  const baseItems: SidebarItem[] = [
    {
      id: 'meetings',
      title: 'Meetings',
      type: 'folder' as const,
      children: [
        // { id: 'team-sync-dec-26', title: 'Team Sync - Dec 26', type: 'file' as const },
        // { id: 'product-review', title: 'Product Review', type: 'file' as const },
        ...meetings.map(meeting => ({ id: meeting.id, title: meeting.title, type: 'file' as const }))
      ]
    },
    {
      id: 'notes',
      title: 'Notes',
      type: 'folder' as const,
      children: [
        { id: 'project-ideas', title: 'Project Ideas', type: 'file' as const },
        { id: 'action-items', title: 'Action Items', type: 'file' as const },
      ]
    }
  ];

  const sidebarItems: SidebarItem[] = baseItems.map(item => {
    if (item.id === 'meetings' && currentMeeting) {
      const newItem: SidebarItem = {
        ...item,
        children: [
          { id: currentMeeting.id, title: currentMeeting.title, type: 'file' as const },
          ...(item.children || []).filter(child => child.id !== currentMeeting.id)
        ]
      };
      return newItem;
    }
    return item;
  });

  const toggleCollapse = () => {
    setIsCollapsed(!isCollapsed);
  };

  // Update current meeting when on home page
  useEffect(() => {
    if (pathname === '/') {
      setCurrentMeeting({ id: 'intro-call', title: 'New Call' });
    }
  }, [pathname]);

  return (
    <SidebarContext.Provider value={{ currentMeeting, setCurrentMeeting, sidebarItems, isCollapsed, toggleCollapse }}>
      {children}
    </SidebarContext.Provider>
  );
}
