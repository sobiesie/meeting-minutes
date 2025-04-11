import { Clock, Users, Calendar, Tag } from 'lucide-react';
import { EditableTitle } from '@/components/EditableTitle';
import PageContent from '../page-content';
import { Summary, } from '@/types';


interface PageProps {
  params: {
    id: string;
  };
}

interface Meeting {
  title: string;
  date: string;
  time?: string;
  attendees?: string[];
  tags: string[];
  content: string;
}

// export function generateStaticParams() {
//   // Return all possible meeting IDs
//   return [
//     { id: 'team-sync-dec-26' },
//     { id: 'product-review' },
//     { id: 'project-ideas' },
//     { id: 'action-items' }
//   ];
// }

// const response = await fetch('http://localhost:5167/get-meetings');
//         const data = await response.json();
//         // Transform the response into the expected format
//         const transformedMeetings = data.map((meeting: any) => ({
//           id: meeting.id,
//           title: meeting.title
//         }));

export async function generateStaticParams() {
  console.log('Fetching meetings from API...');
  const meetings = await fetch('http://localhost:5167/get-meetings', {
    cache: 'no-store',
    headers: {
      'Cache-Control': 'no-cache, no-store, must-revalidate',
      'Pragma': 'no-cache',
      'Expires': '0'
    }
  }).then(res => res.json());
  console.log('Meetings received:', meetings);
  
  // Return array of objects with id property for each meeting
  if (meetings.length > 0) {
    return meetings.map((meeting: { id: string }) => ({
      id: meeting.id
    }));
  } else {
    return [{ id: 'intro-call' }];
  }
}

export default async function MeetingPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  console.log('Fetching meeting details for ID:', id);
  const response = await fetch(`http://localhost:5167/get-meeting/${id}`, {
    cache: 'no-store',
    headers: {
      'Cache-Control': 'no-cache, no-store, must-revalidate',
      'Pragma': 'no-cache',
      'Expires': '0'
    }
  });
  const summaryResponse = await fetch(`http://localhost:5167/get-summary/${id}`, {
    cache: 'no-store',
    headers: {
      'Cache-Control': 'no-cache, no-store, must-revalidate',
      'Pragma': 'no-cache',
      'Expires': '0'
    }
  });
  const summary = await summaryResponse.json();
  const meeting = await response.json();
  
  // Handle case where summary data is null
  const summaryData = summary.data || {};
  const { MeetingName, ...restSummaryData } = summaryData;
  
  // Format the summary data with consistent styling
  const formattedSummary = Object.entries(restSummaryData).reduce((acc: Summary, [key, section]: [string, any]) => {
    acc[key] = {
      title: section?.title || key,
      blocks: (section?.blocks || []).map((block: any) => ({
        ...block,
        type: 'bullet',
        color: 'default',
        content: block.content.trim() // Remove trailing newlines
      }))
    };
    return acc;
  }, {} as Summary);

  console.log('Meeting details received:', meeting);
  const sampleSummary = {
    key_points: { title: "Key Points", blocks: [] },
    action_items: { title: "Action Items", blocks: [] },
    decisions: { title: "Decisions", blocks: [] },
    main_topics: { title: "Main Topics", blocks: [] }
  };
  return <PageContent meeting={meeting} summaryData={formattedSummary || sampleSummary} />
}

  // const [transcripts, setTranscripts] = useState<Transcript[]>([]);


//   // return <div>Meeting {id}</div>;
//   return (
//     <div className="flex flex-col h-screen bg-gray-50">
//       <div className="flex flex-1 overflow-hidden">
//         {/* Left side - Transcript */}
//         <div className="w-1/3 min-w-[300px] border-r border-gray-200 bg-white flex flex-col relative">
//           {/* Title area */}
//           <div className="p-4 border-b border-gray-200">
//             <div className="flex flex-col space-y-3">
//               <div className="flex items-center">
//                 <EditableTitle
//                   title={meetingTitle}
//                   isEditing={isEditingTitle}
//                   onStartEditing={() => setIsEditingTitle(true)}
//                   onFinishEditing={() => setIsEditingTitle(false)}
//                   onChange={handleTitleChange}
//                 />
//               </div>
//               <div className="flex items-center space-x-2">
//                 <button
//                   onClick={handleCopyTranscript}
//                   disabled={transcripts.length === 0}
//                   className={`px-3 py-2 border rounded-md transition-all duration-200 inline-flex items-center gap-2 shadow-sm ${
//                     transcripts.length === 0
//                       ? 'bg-gray-50 border-gray-200 text-gray-400 cursor-not-allowed'
//                       : 'bg-blue-50 border-blue-200 text-blue-700 hover:bg-blue-100 hover:border-blue-300 active:bg-blue-200'
//                   }`}
//                   title={transcripts.length === 0 ? 'No transcript available' : 'Copy Transcript'}
//                 >
//                   <svg xmlns="http://www.w3.org/2000/svg" className="h-4 w-4" viewBox="0 0 24 24" strokeWidth="2" stroke="currentColor" fill="none">
//                     <path strokeLinecap="round" strokeLinejoin="round" d="M15.666 3.888A2.25 2.25 0 0 0 13.5 3h-6a2.25 2.25 0 0 0-2.25 2.25v13.5A2.25 2.25 0 0 0 7.5 21h6a2.25 2.25 0 0 0 2.25-2.25V7.5l-3.75-3.612z" />
//                     <path strokeLinecap="round" strokeLinejoin="round" d="M15 3v3.75a.75.75 0 0 0 .75.75H18" />
//                   </svg>
//                   <span className="text-sm">Copy Transcript</span>
//                 </button>
//                 {showSummary && !isRecording && (
//                   <>
//                     <button
//                       onClick={handleGenerateSummary}
//                       disabled={summaryStatus === 'processing'}
//                       className={`px-3 py-2 border rounded-md transition-all duration-200 inline-flex items-center gap-2 shadow-sm ${
//                         summaryStatus === 'processing'
//                           ? 'bg-yellow-50 border-yellow-200 text-yellow-700'
//                           : transcripts.length === 0
//                           ? 'bg-gray-50 border-gray-200 text-gray-400 cursor-not-allowed'
//                           : 'bg-green-50 border-green-200 text-green-700 hover:bg-green-100 hover:border-green-300 active:bg-green-200'
//                       }`}
//                       title={
//                         summaryStatus === 'processing'
//                           ? 'Generating summary...'
//                           : transcripts.length === 0
//                           ? 'No transcript available'
//                           : 'Generate AI Summary'
//                       }
//                     >
//                       {summaryStatus === 'processing' ? (
//                         <>
//                           <svg className="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
//                             <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
//                             <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
//                           </svg>
//                           <span className="text-sm">Processing...</span>
//                         </>
//                       ) : (
//                         <>
//                           <svg xmlns="http://www.w3.org/2000/svg" className="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
//                             <path strokeLinecap="round" strokeLinejoin="round" d="M13 10V3L4 14h7v7l9-11h-7z" />
//                           </svg>
//                           <span className="text-sm">Generate Note</span>
//                         </>
//                       )}
//                     </button>
//                     <button
//                       onClick={() => setShowModelSettings(true)}
//                       className="px-3 py-2 border rounded-md transition-all duration-200 inline-flex items-center gap-2 shadow-sm bg-gray-50 border-gray-200 text-gray-700 hover:bg-gray-100 hover:border-gray-300 active:bg-gray-200"
//                       title="Model Settings"
//                     >
//                       <svg xmlns="http://www.w3.org/2000/svg" className="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
//                         <path strokeLinecap="round" strokeLinejoin="round" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
//                         <path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
//                       </svg>
//                     </button>
//                   </>
//                 )}
//               </div>
//             </div>
//           </div>

//           {/* Transcript content */}
//           <div className="flex-1 overflow-y-auto pb-32">
//             <TranscriptView transcripts={transcripts} />
//           </div>

//           {/* Recording controls */}
//           <div className="absolute bottom-16 left-1/2 transform -translate-x-1/2 z-10">
//             <div className="bg-white rounded-full shadow-lg flex items-center">
//               <RecordingControls
//                 isRecording={isRecording}
//                 onRecordingStop={() => handleRecordingStop2(true)}
//                 onRecordingStart={handleRecordingStart}
//                 onTranscriptReceived={handleTranscriptUpdate}
//                 barHeights={barHeights}
//               />
//             </div>
//           </div>

//           {/* Model Settings Modal */}
//           {showModelSettings && (
//             <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
//               <div className="bg-white rounded-lg p-6 max-w-md w-full mx-4 shadow-xl">
//                 <div className="flex justify-between items-center mb-4">
//                   <h3 className="text-lg font-semibold text-gray-900">Model Settings</h3>
//                   <button
//                     onClick={() => setShowModelSettings(false)}
//                     className="text-gray-500 hover:text-gray-700"
//                   >
//                     <svg xmlns="http://www.w3.org/2000/svg" className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
//                       <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
//                     </svg>
//                   </button>
//                 </div>

//                 <div className="space-y-4">
//                   <div>
//                     <label className="block text-sm font-medium text-gray-700 mb-1">
//                       Summarization Model
//                     </label>
//                     <div className="flex space-x-2">
//                       <select
//                         className="px-3 py-2 text-sm bg-white border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
//                         value={modelConfig.provider}
//                         onChange={(e) => {
//                           const provider = e.target.value as ModelConfig['provider'];
//                           setModelConfig({
//                             ...modelConfig,
//                             provider,
//                             model: modelOptions[provider][0]
//                           });
//                         }}
//                       >
//                         <option value="claude">Claude</option>
//                         <option value="groq">Groq</option>
//                         <option value="ollama">Ollama</option>
//                       </select>

//                       <select
//                         className="flex-1 px-3 py-2 text-sm bg-white border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
//                         value={modelConfig.model}
//                         onChange={(e) => setModelConfig(prev => ({ ...prev, model: e.target.value }))}
//                       >
//                         {modelOptions[modelConfig.provider].map(model => (
//                           <option key={model} value={model}>
//                             {model}
//                           </option>
//                         ))}
//                       </select>
//                     </div>
//                   </div>
//                   {modelConfig.provider === 'ollama' && (
//                     <div>
//                       <h4 className="text-lg font-bold mb-4">Available Ollama Models</h4>
//                       {error && (
//                         <div className="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4">
//                           {error}
//                         </div>
//                       )}
//                       <div className="grid gap-4 max-h-[400px] overflow-y-auto pr-2">
//                         {models.map((model) => (
//                           <div 
//                             key={model.id}
//                             className={`bg-white p-4 rounded-lg shadow cursor-pointer transition-colors ${
//                               modelConfig.model === model.name ? 'ring-2 ring-blue-500 bg-blue-50' : 'hover:bg-gray-50'
//                             }`}
//                             onClick={() => setModelConfig(prev => ({ ...prev, model: model.name }))}
//                           >
//                             <h3 className="font-bold">{model.name}</h3>
//                             <p className="text-gray-600">Size: {model.size}</p>
//                             <p className="text-gray-600">Modified: {model.modified}</p>
//                           </div>
//                         ))}
//                       </div>
//                     </div>
//                   )}
//                 </div>

//                 <div className="mt-6 flex justify-end">
//                   <button
//                     onClick={() => setShowModelSettings(false)}
//                     className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
//                   >
//                     Done
//                   </button>
//                 </div>
//               </div>
//             </div>
//           )}
//         </div>

//         {/* Right side - AI Summary */}
//         <div className="flex-1 overflow-y-auto bg-white">
//           {isSummaryLoading ? (
//             <div className="flex items-center justify-center h-full">
//               <div className="text-center">
//                 <div className="inline-block animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-500 mb-4"></div>
//                 <p className="text-gray-600">Generating AI Summary...</p>
//               </div>
//             </div>
//           ) : showSummary && (
//             <div className="max-w-4xl mx-auto p-6">
//               {summaryResponse && (
//                 <div className="fixed bottom-0 left-0 right-0 bg-white shadow-lg p-4 max-h-1/3 overflow-y-auto">
//                   <h3 className="text-lg font-semibold mb-2">Meeting Summary</h3>
//                   <div className="grid grid-cols-2 gap-4">
//                     <div className="bg-white p-4 rounded-lg shadow-sm">
//                       <h4 className="font-medium mb-1">Key Points</h4>
//                       <ul className="list-disc pl-4">
//                         {summaryResponse.summary.key_points.blocks.map((block, i) => (
//                           <li key={i} className="text-sm">{block.content}</li>
//                         ))}
//                       </ul>
//                     </div>
//                     <div className="bg-white p-4 rounded-lg shadow-sm mt-4">
//                       <h4 className="font-medium mb-1">Action Items</h4>
//                       <ul className="list-disc pl-4">
//                         {summaryResponse.summary.action_items.blocks.map((block, i) => (
//                           <li key={i} className="text-sm">{block.content}</li>
//                         ))}
//                       </ul>
//                     </div>
//                     <div className="bg-white p-4 rounded-lg shadow-sm mt-4">
//                       <h4 className="font-medium mb-1">Decisions</h4>
//                       <ul className="list-disc pl-4">
//                         {summaryResponse.summary.decisions.blocks.map((block, i) => (
//                           <li key={i} className="text-sm">{block.content}</li>
//                         ))}
//                       </ul>
//                     </div>
//                     <div className="bg-white p-4 rounded-lg shadow-sm mt-4">
//                       <h4 className="font-medium mb-1">Main Topics</h4>
//                       <ul className="list-disc pl-4">
//                         {summaryResponse.summary.main_topics.blocks.map((block, i) => (
//                           <li key={i} className="text-sm">{block.content}</li>
//                         ))}
//                       </ul>
//                     </div>
//                   </div>
//                   {summaryResponse.raw_summary ? (
//                     <div className="mt-4">
//                       <h4 className="font-medium mb-1">Full Summary</h4>
//                       <p className="text-sm whitespace-pre-wrap">{summaryResponse.raw_summary}</p>
//                     </div>
//                   ) : null}
//                 </div>
//               )}
//               <div className="flex-1 overflow-y-auto p-4">
//                 <AISummary 
//                   summary={aiSummary} 
//                   status={summaryStatus} 
//                   error={summaryError}
//                   onSummaryChange={(newSummary) => setAiSummary(newSummary)}
//                   onRegenerateSummary={handleRegenerateSummary}
//                 />
//               </div>
//               {summaryStatus !== 'idle' && (
//                 <div className={`mt-4 p-4 rounded-lg ${
//                   summaryStatus === 'error' ? 'bg-red-100 text-red-700' :
//                   summaryStatus === 'completed' ? 'bg-green-100 text-green-700' :
//                   'bg-blue-100 text-blue-700'
//                 }`}>
//                   <p className="text-sm font-medium">{getSummaryStatusMessage(summaryStatus)}</p>
//                 </div>
//               )}
//             </div>
//           )}
//         </div>
//       </div>
//     </div>
//   );
// }


// const MeetingPage = ({ params }: PageProps) => {
//   // This would normally come from your database
//   const sampleData: Record<string, Meeting> = {
//     'team-sync-dec-26': {
//       title: 'Team Sync - Dec 26',
//       date: '2024-12-26',
//       time: '10:00 AM - 11:00 AM',
//       attendees: ['John Doe', 'Jane Smith', 'Mike Johnson'],
//       tags: ['Team Sync', 'Weekly', 'Product'],
//       content: `
// # Meeting Summary
// Team sync discussion about Q1 2024 goals and current project status.

// ## Agenda Items
// 1. Project Status Updates
// 2. Q1 2024 Planning
// 3. Team Concerns & Feedback

// ## Key Decisions
// - Prioritized mobile app development for Q1
// - Scheduled weekly design reviews
// - Added two new features to the roadmap

// ## Action Items
// - [ ] John: Create project timeline
// - [ ] Jane: Schedule design review meetings
// - [ ] Mike: Update documentation

// ## Notes
// - Discussed current project bottlenecks
// - Reviewed customer feedback from last release
// - Planned resource allocation for upcoming sprint
//       `
//     },
//     'product-review': {
//       title: 'Product Review',
//       date: '2024-12-26',
//       time: '2:00 PM - 3:00 PM',
//       attendees: ['Sarah Wilson', 'Tom Brown', 'Alex Chen'],
//       tags: ['Product', 'Review', 'Quarterly'],
//       content: `
// # Product Review Meeting

// ## Current Status
// - User engagement up by 25%
// - New feature adoption rate at 80%
// - Customer satisfaction score: 4.5/5

// ## Key Metrics
// 1. Monthly Active Users: 50k
// 2. Average Session Duration: 15 mins
// 3. Conversion Rate: 12%

// ## Action Items
// - [ ] Sarah: Prepare Q1 roadmap
// - [ ] Tom: Analyze user feedback
// - [ ] Alex: Update product metrics dashboard

// ## Next Steps
// - Schedule follow-up meetings
// - Review competitor analysis
// - Plan feature prioritization workshop
//       `
//     },
//     'project-ideas': {
//       title: 'Project Ideas',
//       date: '2024-12-26',
//       time: '4:00 PM - 5:00 PM',
//       attendees: ['Emily Lee', 'David Kim', 'Olivia Taylor'],
//       tags: ['Project', 'Ideas', 'Brainstorming'],
//       content: `
// # Project Ideas Meeting

// ## Current Status
// - Reviewed current project pipeline
// - Discussed new project ideas

// ## Key Ideas
// 1. Develop a new mobile app for customer engagement
// 2. Create a machine learning model for predictive analytics
// 3. Design a new website for marketing campaigns

// ## Action Items
// - [ ] Emily: Research mobile app development frameworks
// - [ ] David: Investigate machine learning libraries
// - [ ] Olivia: Sketch new website design concepts

// ## Next Steps
// - Schedule follow-up meetings to discuss project ideas
// - Review project proposals and prioritize projects
// - Plan project kick-off meetings
//       `
//     },
//     'action-items': {
//       title: 'Action Items Review',
//       date: '2024-12-26',
//       time: '5:00 PM - 6:00 PM',
//       attendees: ['Project Team'],
//       tags: ['Tasks', 'Review', 'Planning'],
//       content: `
// # Action Items Review Meeting

// ## Progress Review
// - Reviewed completed tasks from last week
// - Discussed blockers and challenges
// - Updated task priorities

// ## Key Decisions
// 1. Prioritized security fixes for immediate deployment
// 2. Scheduled dependency updates for next sprint
// 3. Assigned new tasks to team members

// ## Next Steps
// - Daily progress updates
// - Weekly review meetings
// - Monthly planning sessions
//       `
//     }
//   };

//   const meeting = sampleData[params.id as keyof typeof sampleData];

//   if (!meeting) {
//     return <div className="p-8">Meeting not found</div>;
//   }

//   return (
//     <div className="p-8 max-w-4xl mx-auto">
//       <div className="mb-8">
//         <h1 className="text-3xl font-bold mb-4">{meeting.title}</h1>
        
//         <div className="flex flex-wrap gap-4 text-gray-600">
//           {meeting.date && (
//             <div className="flex items-center gap-1">
//               <Calendar className="w-4 h-4" />
//               <span>{meeting.date}</span>
//             </div>
//           )}
          
//           {meeting.time && (
//             <div className="flex items-center gap-1">
//               <Clock className="w-4 h-4" />
//               <span>{meeting.time}</span>
//             </div>
//           )}
          
//           {meeting.attendees && (
//             <div className="flex items-center gap-1">
//               <Users className="w-4 h-4" />
//               <span>{meeting.attendees.join(', ')}</span>
//             </div>
//           )}
//         </div>

//         <div className="flex gap-2 mt-4">
//           {meeting.tags.map((tag) => (
//             <div key={tag} className="flex items-center gap-1 bg-blue-100 text-blue-800 px-2 py-1 rounded-full text-sm">
//               <Tag className="w-3 h-3" />
//               {tag}
//             </div>
//           ))}
//         </div>
//       </div>

//       <div className="prose prose-blue max-w-none">
//         <div dangerouslySetInnerHTML={{ __html: meeting.content.split('\n').map(line => {
//           if (line.startsWith('# ')) {
//             return `<h1>${line.slice(2)}</h1>`;
//           } else if (line.startsWith('## ')) {
//             return `<h2>${line.slice(3)}</h2>`;
//           } else if (line.startsWith('- ')) {
//             return `<li>${line.slice(2)}</li>`;
//           }
//           return line;
//         }).join('\n') }} />
//       </div>
//     </div>
//   );
// };

// export default MeetingPage;
