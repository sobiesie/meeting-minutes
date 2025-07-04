'use client';

import React, { useState, useMemo, useEffect, useCallback } from 'react';
import { ChevronDown, ChevronRight, File, Settings, ChevronLeftCircle, ChevronRightCircle, Calendar, StickyNote, Home, Delete, Mic, Square, Plus, Search } from 'lucide-react';
import { useRouter } from 'next/navigation';
import { useSidebar } from './SidebarProvider';
import type { CurrentMeeting } from '@/components/Sidebar/SidebarProvider';
import { ConfirmationModal } from '../ConfirmationModel/confirmation-modal';
import {  ModelConfig } from '@/components/ModelSettingsModal';
import { SettingTabs } from '../SettingTabs';
import { TranscriptModelProps } from '@/components/TranscriptSettings';


import {
  Dialog,
  DialogContent,
  DialogTrigger,
  DialogFooter,
} from "@/components/ui/dialog"
import { MessageToast } from '../MessageToast';

interface SidebarItem {
  id: string;
  title: string;
  type: 'folder' | 'file';
  children?: SidebarItem[];
}

const Sidebar: React.FC = () => {
  const router = useRouter();
  const { 
    currentMeeting, 
    setCurrentMeeting, 
    sidebarItems, 
    isCollapsed, 
    toggleCollapse, 
    isMeetingActive,
    isRecording,
    handleRecordingToggle,
    searchTranscripts,
    searchResults,
    isSearching,
    meetings,
    setMeetings
  } = useSidebar();
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set(['meetings', 'notes']));
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [showModelSettings, setShowModelSettings] = useState(false);
  const [modelConfig, setModelConfig] = useState<ModelConfig>({
    provider: 'ollama',
    model: 'llama3.2:latest',
    whisperModel: 'large-v3',
    apiKey: null
  });
  const [transcriptModelConfig, setTranscriptModelConfig] = useState<TranscriptModelProps>({
    provider: 'localWhisper',
    model: 'large-v3',
  });
  const [settingsSaveSuccess, setSettingsSaveSuccess] = useState<boolean | null>(null);
  
  // Ensure 'meetings' folder is always expanded
  useEffect(() => {
    if (!expandedFolders.has('meetings')) {
      const newExpanded = new Set(expandedFolders);
      newExpanded.add('meetings');
      setExpandedFolders(newExpanded);
    }
  }, [expandedFolders]);

  useEffect(() => {
    if (settingsSaveSuccess !== null) {
      const timer = setTimeout(() => {
        setSettingsSaveSuccess(null);
      }, 3000);
    }
  }, [settingsSaveSuccess]);


  const [deleteModalState, setDeleteModalState] = useState<{ isOpen: boolean; itemId: string | null }>({ isOpen: false, itemId: null });
  
  // Handle model config save
  const handleSaveModelConfig = async (config: ModelConfig) => {
    try {
      const response = await fetch('http://localhost:5167/save-model-config', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(config),
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      setModelConfig(config);
      console.log('Model config saved successfully');
      setSettingsSaveSuccess(true);
    } catch (error) {
      console.error('Error saving model config:', error);
      setSettingsSaveSuccess(false);
    }
  };

  const handleSaveTranscriptConfig = async (updatedConfig?: TranscriptModelProps) => {
    try {
      const configToSave = updatedConfig || transcriptModelConfig;
      const payload = {
        provider: configToSave.provider,
        model: configToSave.model,
        apiKey: configToSave.apiKey ?? null
      };
      console.log('Saving transcript config with payload:', payload);
      
      const response = await fetch('http://localhost:5167/save-transcript-config', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(payload)
      });

      if (!response.ok) {
        const errorData = await response.json();
        console.error('Save transcript config failed:', errorData);
        console.error('Response status:', response.status);
        throw new Error(errorData.error || 'Failed to save transcript config');
      }

      const responseData = await response.json();
      console.log('Save transcript config success:', responseData);
      setSettingsSaveSuccess(true);
    } catch (error) {
      console.error('Failed to save transcript config:', error);
      setSettingsSaveSuccess(false);
    }
  };
  
  // Handle search input changes
  const handleSearchChange = useCallback(async (value: string) => {
    setSearchQuery(value);
    
    // If search query is empty, just return to normal view
    if (!value.trim()) return;
    
    // Search through transcripts
    await searchTranscripts(value);
    
    // Make sure the meetings folder is expanded when searching
    if (!expandedFolders.has('meetings')) {
      const newExpanded = new Set(expandedFolders);
      newExpanded.add('meetings');
      setExpandedFolders(newExpanded);
    }
  }, [expandedFolders, searchTranscripts]);
  
  // Combine search results with sidebar items
  const filteredSidebarItems = useMemo(() => {
    if (!searchQuery.trim()) return sidebarItems;
    
    // If we have search results, highlight matching meetings
    if (searchResults.length > 0) {
      // Get the IDs of meetings that matched in transcripts
      const matchedMeetingIds = new Set(searchResults.map(result => result.id));
      
      return sidebarItems
        .map(folder => {
          // Always include folders in the results
          if (folder.type === 'folder') {
            if (!folder.children) return folder;
            
            // Filter children based on search results or title match
            const filteredChildren = folder.children.filter(item => {
              // Include if the meeting ID is in our search results
              if (matchedMeetingIds.has(item.id)) return true;
              
              // Or if the title matches the search query
              return item.title.toLowerCase().includes(searchQuery.toLowerCase());
            });
            
            return {
              ...folder,
              children: filteredChildren
            };
          }
          
          // For non-folder items, check if they match the search
          return (matchedMeetingIds.has(folder.id) || 
                 folder.title.toLowerCase().includes(searchQuery.toLowerCase())) 
                 ? folder : undefined;
        })
        .filter((item): item is SidebarItem => item !== undefined); // Type-safe filter
    } else {
      // Fall back to title-only filtering if no transcript results
      return sidebarItems
        .map(folder => {
          // Always include folders in the results
          if (folder.type === 'folder') {
            if (!folder.children) return folder;
            
            // Filter children based on search query
            const filteredChildren = folder.children.filter(item => 
              item.title.toLowerCase().includes(searchQuery.toLowerCase())
            );
            
            return {
              ...folder,
              children: filteredChildren
            };
          }
          
          // For non-folder items, check if they match the search
          return folder.title.toLowerCase().includes(searchQuery.toLowerCase()) ? folder : undefined;
        })
        .filter((item): item is SidebarItem => item !== undefined); // Type-safe filter
    }
  }, [sidebarItems, searchQuery, searchResults, expandedFolders]);


  const handleDelete = async (itemId: string) => {
    console.log('Deleting item:', itemId);
    const payload = {
      meeting_id: itemId
    };
    
    const response = await fetch('http://localhost:5167/delete-meeting', {
      cache: 'no-store',
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    });
    
    if (response.ok) {
      console.log('Meeting deleted successfully');
      const updatedMeetings = meetings.filter((m: CurrentMeeting) => m.id !== itemId);
      setMeetings(updatedMeetings);
      
      // If deleting the active meeting, navigate to home
      if (currentMeeting?.id === itemId) {
        setCurrentMeeting({ id: 'intro-call', title: '+ New Call' });
        router.push('/');
      }
    } else {
      console.error('Failed to delete meeting');
    }
  };
  
  const handleDeleteConfirm = () => {
    if (deleteModalState.itemId) {
      handleDelete(deleteModalState.itemId);
    }
    setDeleteModalState({ isOpen: false, itemId: null });
  };

  const toggleFolder = (folderId: string) => {
    // Prevent 'meetings' folder from being closed
    if (folderId === 'meetings') {
      // Only allow adding 'meetings' to expanded folders, never removing
      const newExpanded = new Set(expandedFolders);
      newExpanded.add(folderId);
      setExpandedFolders(newExpanded);
      return;
    }
    
    // Normal toggle behavior for other folders
    const newExpanded = new Set(expandedFolders);
    if (newExpanded.has(folderId)) {
      newExpanded.delete(folderId);
    } else {
      newExpanded.add(folderId);
    }
    setExpandedFolders(newExpanded);
  };

  const renderCollapsedIcons = () => {
    if (!isCollapsed) return null;

    return (
      <div className="flex flex-col items-center space-y-4 mt-4">
        {/* New Call button for collapsed sidebar */}
        <span className="text-lg text-center border rounded-full bg-blue-50 border-white font-semibold text-gray-700 mb-2 block items-center">
          <span className='m-3'>Me</span>
        </span>
        <button
          onClick={handleRecordingToggle}
          disabled={isRecording}
          className={`p-2 ${isRecording ? 'bg-red-500 cursor-not-allowed' : 'bg-red-500 hover:bg-red-600'} rounded-full transition-colors shadow-sm`}
          title={isRecording ? "Recording in progress..." : "Start New Call"}
        >
          {isRecording ? (
            <Square className="w-5 h-5 text-white" />
          ) : (
            <Mic className="w-5 h-5 text-white" />
          )}
        </button>
        
        <button
          onClick={() => router.push('/')}
          className="p-2 rounded-lg hover:bg-gray-100 transition-colors"
          title="Home"
        >
          <Home className="w-5 h-5 text-gray-600" />
        </button>
        
        <button
          onClick={() => {
            if (isCollapsed) toggleCollapse();
            toggleFolder('meetings');
          }}
          className="p-2 hover:bg-gray-100 rounded-md transition-colors"
          title="Meetings"
        >
          <StickyNote className="w-5 h-5 text-gray-600" />
        </button>
        <button
          onClick={() => setShowModelSettings(true)}
          className="p-2 rounded-lg hover:bg-gray-100 transition-colors"
          title="Settings"
        >
          <Settings className="w-5 h-5 text-gray-600" />
        </button>
        {/* <button
          onClick={() => {
            if (isCollapsed) toggleCollapse();
            toggleFolder('notes');
          }}
          className="p-2 hover:bg-gray-100 rounded-md transition-colors"
          title="Notes"
        >
          <StickyNote className="w-5 h-5 text-gray-600" />
        </button> */}
      </div>
    );
  };

  // Find matching transcript snippet for a meeting item
  const findMatchingSnippet = (itemId: string) => {
    if (!searchQuery.trim() || !searchResults.length) return null;
    return searchResults.find(result => result.id === itemId);
  };

  const renderItem = (item: SidebarItem, depth = 0) => {
    const isExpanded = expandedFolders.has(item.id);
    const paddingLeft = `${depth * 12 + 12}px`;
    const isActive = item.type === 'file' && currentMeeting?.id === item.id;
    const isMeetingItem = item.id.includes('-') && !item.id.startsWith('intro-call');
    const isDisabled = isMeetingActive && isMeetingItem;
    
    // Check if this item has a matching transcript snippet
    const matchingResult = isMeetingItem ? findMatchingSnippet(item.id) : null;
    const hasTranscriptMatch = !!matchingResult;

    if (isCollapsed) return null;

    return (
      <div key={item.id}>
        <div
          className={`flex items-center px-3 py-2 my-0.5 rounded-md transition-all duration-150 text-sm group ${
            isActive ? 'bg-blue-50 text-blue-700 font-medium shadow-sm' : 
            hasTranscriptMatch ? 'bg-yellow-50' : 'hover:bg-gray-50'
          } ${
            isDisabled ? 'opacity-60 cursor-not-allowed' : 'cursor-pointer'
          }`}
          style={{ paddingLeft }}
          onClick={() => {
            if (item.type === 'folder') {
              toggleFolder(item.id);
            } else {
              if (isDisabled) {
                return;
              }
              
              setCurrentMeeting({ id: item.id, title: item.title });
              const basePath = item.id.startsWith('intro-call') ? '/' : 
                item.id.includes('-') ? '/meeting-details' : `/notes/${item.id}`;
              router.push(basePath);
            }
          }}
        >
          {item.type === 'folder' ? (
            <>
              <div className="flex items-center bg-gray-50 p-1 rounded mr-2">
                {item.id === 'meetings' ? (
                  <Calendar className="w-4 h-4 text-gray-700" />
                ) : item.id === 'notes' ? (
                  <StickyNote className="w-4 h-4 text-gray-700" />
                ) : null}
              </div>
              {isExpanded ? (
                <ChevronDown className="w-4 h-4 mr-1 text-gray-500" />
              ) : (
                <ChevronRight className="w-4 h-4 mr-1 text-gray-500" />
              )}
              <span className="font-medium">{item.title}</span>
              {searchQuery && item.id === 'meetings' && isSearching && (
                <span className="ml-2 text-xs text-blue-500 animate-pulse">Searching...</span>
              )}
            </>
          ) : (
            <div className="flex flex-col w-full">
              <div className="flex items-start justify-between w-full">
                <div className="flex items-start">
                  {isMeetingItem ? (
                    <div className={`flex-shrink-0 flex items-center justify-center w-6 h-6 rounded-full mr-2 mt-0.5 ${
                      isDisabled ? 'bg-gray-100' : 
                      hasTranscriptMatch ? 'bg-yellow-100' : 'bg-blue-100'}`}>
                      <File className={`w-3.5 h-3.5 ${
                        isDisabled ? 'text-gray-400' : 
                        hasTranscriptMatch ? 'text-yellow-600' : 'text-blue-600'}`} />
                    </div>
                  ) : (
                    <div className="flex-shrink-0 flex items-center justify-center w-6 h-6 rounded-full mr-2 mt-0.5 bg-green-100">
                      <Plus className="w-3.5 h-3.5 text-green-600" />
                    </div>
                  )}
                  <span className={`break-words pr-6 leading-tight ${isDisabled ? 'text-gray-400' : ''}`}>{item.title}</span>
                </div>
                {isMeetingItem && !isDisabled && (
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      setDeleteModalState({ isOpen: true, itemId: item.id });
                    }}
                    className="opacity-0 group-hover:opacity-100 hover:text-red-600 p-1 rounded-md hover:bg-red-50 transition-opacity duration-200 flex-shrink-0 ml-1"
                    aria-label="Delete meeting"
                  >
                    <Delete className="w-4 h-4" />
                  </button>
                )}
              </div>
              
              {/* Show transcript match snippet if available */}
              {hasTranscriptMatch && (
                <div className="mt-1 ml-8 text-xs text-gray-500 bg-yellow-50 p-1.5 rounded border border-yellow-100 line-clamp-2">
                  <span className="font-medium text-yellow-600">Match:</span> {matchingResult.matchContext}
                </div>
              )}
            </div>
          )}
        </div>
        {item.type === 'folder' && isExpanded && item.children && (
          <div className="ml-1 border-l border-gray-100">
            {item.children.map(child => renderItem(child, depth + 1))}
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="fixed top-0 left-0 h-screen z-40">
      {/* Floating collapse button */}
      <button
        onClick={toggleCollapse}
        className="absolute -right-6 top-20 z-50 p-1 bg-white hover:bg-gray-100 rounded-full shadow-lg border"
        style={{ transform: 'translateX(50%)' }}
      >
        {isCollapsed ? (
          <ChevronRightCircle className="w-6 h-6" />
        ) : (
          <ChevronLeftCircle className="w-6 h-6" />
        )}
      </button>

      <div 
        className={`h-screen bg-white border-r shadow-sm flex flex-col transition-all duration-300 ${
          isCollapsed ? 'w-16' : 'w-64'
        }`}
      >
        {/* Header with traffic light spacing */}
        <div className="h-22 flex items-center border-b">
        
          {/* Title container */}
          
          
          
          <div className="flex-1">
            {!isCollapsed && (
              <div className="p-3">
                <span className="text-lg text-center border rounded-full bg-blue-50 border-white font-semibold text-gray-700 mb-2 block items-center">
                  <span>Meetily</span>
                </span>
                
                <div className="relative mb-1">
              <div className="absolute inset-y-0 left-0 pl-2 flex items-center pointer-events-none">
                <Search className="h-3.5 w-3.5 text-gray-400" />
              </div>
              <input
                type="text"
                placeholder="Search meeting content..."
                value={searchQuery}
                onChange={(e) => handleSearchChange(e.target.value)}
                className="block w-full pl-8 pr-3 py-1.5 border border-gray-200 rounded-md text-xs placeholder-gray-400 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
              />
              {searchQuery && (
                <button
                  onClick={() => handleSearchChange('')}
                  className="absolute inset-y-0 right-0 pr-2 flex items-center text-gray-400 hover:text-gray-500"
                >
                  <span className="text-xs">×</span>
                </button>
              )}
            </div>
           
            </div>
            )}
          </div>
        </div>

        {/* Main content */}
        <div className="flex-1 overflow-y-auto custom-scrollbar">
          {!isCollapsed && (
            <div className="p-3 mb-3 text-lg font-semibold items-center">
              Meeting Notes
            </div>
          )}
          {renderCollapsedIcons()}
          <div className="px-2">
            {filteredSidebarItems.map(item => renderItem(item))}
          </div>
        </div>

        {/* Footer */}
        {!isCollapsed && (
          
          <div className="p-2 border-t border-gray-100">
            <button
                onClick={handleRecordingToggle}
                disabled={isRecording}
                className={`w-full flex items-center justify-center px-3 py-2 text-sm font-medium text-white ${isRecording ? 'bg-red-300 cursor-not-allowed' : 'bg-red-500 hover:bg-red-600'} rounded-lg transition-colors shadow-sm`}
              >
                {isRecording ? (
                  <>
                    <Square className="w-4 h-4 mr-2" />
                    <span>Recording in progress...</span>
                  </>
                ) : (
                  <>
                    <Mic className="w-4 h-4 mr-2" />
                    <span>Start Recording</span>
                  </>
                )}
              </button>
        
              <Dialog>
                <DialogTrigger className='w-full'>
                  <button
                  onClick={() => setShowModelSettings(true)}
                  className="w-full flex items-center justify-center px-3 py-1.5 mt-1 mb-1 text-sm font-medium text-gray-700 bg-gray-200 hover:bg-gray-200 rounded-lg transition-colors shadow-sm"
                  >
                    <Settings className="w-4 h-4 mr-2" />
                    <span>Settings</span>
                  </button>
                </DialogTrigger>
                <DialogContent>
                  <SettingTabs
                    modelConfig={modelConfig}
                    setModelConfig={setModelConfig}
                    onSave={handleSaveModelConfig}
                    transcriptModelConfig={transcriptModelConfig}
                    setTranscriptModelConfig={setTranscriptModelConfig}
                    onSaveTranscript={handleSaveTranscriptConfig}
                    setSaveSuccess={setSettingsSaveSuccess}
                  />
                  <DialogFooter>
                    {settingsSaveSuccess !== null && (
                      <MessageToast message={settingsSaveSuccess ? 'Settings saved successfully' : 'Failed to save settings'} type={settingsSaveSuccess ? 'success' : 'error'} />
                    )}
                  </DialogFooter>

                </DialogContent>

              </Dialog>
              <div className="w-full flex items-center justify-center px-3 py-1 text-xs text-gray-400">
              v0.0.5 - Pre Release
            </div>
          </div>
        )}
      </div>

      {/* Confirmation Modal for Delete */}
      <ConfirmationModal
        isOpen={deleteModalState.isOpen}
        text="Are you sure you want to delete this meeting? This action cannot be undone."
        onConfirm={handleDeleteConfirm}
        onCancel={() => setDeleteModalState({ isOpen: false, itemId: null })}
      />

      
    </div>
  );
};

export default Sidebar;
