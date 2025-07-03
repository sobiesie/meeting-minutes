import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { ModelConfig, ModelSettingsModal } from "./ModelSettingsModal"
import { TranscriptSettings, TranscriptModelProps } from "./TranscriptSettings"

interface SettingTabsProps {
    showModelSettings: boolean;
    setShowModelSettings: (show: boolean) => void;
    modelConfig: ModelConfig;
    setModelConfig: (config: ModelConfig | ((prev: ModelConfig) => ModelConfig)) => void;
    onSave: (config: ModelConfig) => void;
    transcriptModelConfig: TranscriptModelProps;
    setTranscriptModelConfig: (config: TranscriptModelProps) => void;
    onSaveTranscript: (config: TranscriptModelProps) => void;
}

export function SettingTabs({ showModelSettings, setShowModelSettings, modelConfig, setModelConfig, onSave, transcriptModelConfig, setTranscriptModelConfig, onSaveTranscript }: SettingTabsProps) {
    return (
        <Tabs defaultValue="account" className="w-[400px]">
  <TabsList>
    <TabsTrigger value="account">Account</TabsTrigger>
    <TabsTrigger value="password">Password</TabsTrigger>
  </TabsList>
  <TabsContent value="account">
    <ModelSettingsModal
// showModelSettings={showModelSettings}
    // setShowModelSettings={setShowModelSettings}
modelConfig={modelConfig}
setModelConfig={setModelConfig}
onSave={onSave}
/>
  </TabsContent>
  <TabsContent value="password">
    <TranscriptSettings
    transcriptModelConfig={transcriptModelConfig}
    setTranscriptModelConfig={setTranscriptModelConfig}
    onSave={onSaveTranscript}
  />
  </TabsContent>
</Tabs>
    )
}


