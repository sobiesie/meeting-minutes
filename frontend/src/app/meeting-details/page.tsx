"use client"
import { useSidebar } from "@/components/Sidebar/SidebarProvider";
import { useState, useEffect } from "react";
import { Transcript, Summary } from "@/types";
import PageContent from "./page-content";
import { useRouter } from "next/navigation";
import Analytics from "@/lib/analytics";

interface MeetingDetailsResponse {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
  transcripts: Transcript[];
}

const sampleSummary: Summary = {
  key_points: { title: "Key Points", blocks: [] },
  action_items: { title: "Action Items", blocks: [] },
  decisions: { title: "Decisions", blocks: [] },
  main_topics: { title: "Main Topics", blocks: [] }
};

export default function MeetingDetails() {
  const { currentMeeting , serverAddress} = useSidebar();
  const router = useRouter();
  const [meetingDetails, setMeetingDetails] = useState<MeetingDetailsResponse | null>(null);
  const [meetingSummary, setMeetingSummary] = useState<Summary|null>(null);
  const [error, setError] = useState<string | null>(null);

  // Combined effect to handle meeting data fetching and state management
  useEffect(() => {
    // Reset states when currentMeeting changes
    setMeetingDetails(null);
    setMeetingSummary(null);
    setError(null);

    if (!currentMeeting?.id || currentMeeting.id === 'intro-call') {
      setError("No meeting selected");
      Analytics.trackPageView('meeting_details');
      return;
    }

    // Create AbortController for request cancellation
    const abortController = new AbortController();
    let isActive = true;

    const fetchMeetingData = async () => {
      let detailsResponse: PromiseSettledResult<Response> | undefined;
      let summaryResponse: PromiseSettledResult<Response> | undefined;
      
      try {
        // Fetch meeting details
        [detailsResponse, summaryResponse] = await Promise.allSettled([
          fetch(`${serverAddress}/get-meeting/${currentMeeting.id}`, {
            cache: 'no-store',
            signal: abortController.signal
          }),
          fetch(`${serverAddress}/get-summary/${currentMeeting.id}`, {
            cache: 'no-store',
            signal: abortController.signal
          })
        ]);

        // Handle meeting details response
        if (detailsResponse.status === 'fulfilled' && detailsResponse.value.ok) {
          const data = await detailsResponse.value.json();
          if (isActive) {
            console.log('Meeting details:', data);
            setMeetingDetails(data);
          }
        } else if (detailsResponse.status === 'fulfilled') {
          throw new Error('Failed to fetch meeting details');
        }

        // Handle summary response
        if (summaryResponse.status === 'fulfilled') {
          if (summaryResponse.value.ok) {
            const summary = await summaryResponse.value.json();
            if (isActive) {
              const summaryData = summary.data || {};
              const { MeetingName, ...restSummaryData } = summaryData;
              const formattedSummary = Object.entries(restSummaryData).reduce((acc: Summary, [key, section]: [string, any]) => {
                acc[key] = {
                  title: section?.title || key,
                  blocks: (section?.blocks || []).map((block: any) => ({
                    ...block,
                    type: 'bullet',
                    color: 'default',
                    content: block.content.trim()
                  }))
                };
                return acc;
              }, {} as Summary);
              setMeetingSummary(formattedSummary);
            }
          } else if (summaryResponse.value.status === 404) {
            // Summary not found in database - this is expected for meetings without summaries
            console.log(`No summary found for meeting ${currentMeeting.id} - summary not generated yet`);
            if (isActive) {
              setMeetingSummary(sampleSummary);
            }
          } else {
            // Other HTTP errors
            console.log(`Failed to fetch summary for meeting ${currentMeeting.id}: ${summaryResponse.value.status}`);
            if (isActive) {
              setMeetingSummary(sampleSummary);
            }
          }
        } else {
          // Network or other fetch errors
          console.log(`Summary fetch rejected for meeting ${currentMeeting.id}:`, summaryResponse.reason);
          if (isActive) {
            setMeetingSummary(sampleSummary);
          }
        }
      } catch (error) {
        if (error instanceof Error && error.name === 'AbortError') {
          console.log('Request aborted');
          return;
        }
        console.error('Error fetching meeting data:', error);
        if (isActive) {
          // Only set error for critical failures (meeting details), not summary failures
          if (detailsResponse && (detailsResponse.status === 'rejected' || 
              (detailsResponse.status === 'fulfilled' && !detailsResponse.value.ok))) {
            setError("Failed to load meeting details");
          } else {
            // Meeting details loaded successfully, just log summary issue
            console.log('Meeting details loaded, but summary fetch failed - this is acceptable');
          }
        }
      }
    };

    fetchMeetingData();

    // Cleanup function
    return () => {
      isActive = false;
      abortController.abort();
    };
  }, [currentMeeting?.id, serverAddress]);

  // if (error) {
  //   return (
  //     <div className="flex items-center justify-center h-screen">
  //       <div className="text-center">
  //         <p className="text-red-500 mb-4">{error}</p>
  //         <button
  //           onClick={() => router.push('/')}
  //           className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
  //         >
  //           Go Back
  //         </button>
  //       </div>
  //     </div>
  //   );
  // }

  if (!meetingDetails || !meetingSummary) {
    return <div className="flex items-center justify-center h-screen">Loading...</div>;
  }

  return <PageContent meeting={meetingDetails} summaryData={meetingSummary} />;
}