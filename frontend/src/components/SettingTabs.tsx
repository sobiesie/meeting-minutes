import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { ModelConfig, ModelSettingsModal } from "./ModelSettingsModal"

interface SettingTabsProps {
    showModelSettings: boolean;
    setShowModelSettings: (show: boolean) => void;
    modelConfig: ModelConfig;
    setModelConfig: (config: ModelConfig | ((prev: ModelConfig) => ModelConfig)) => void;
    onSave: (config: ModelConfig) => void;
}

export function SettingTabs({ showModelSettings, setShowModelSettings, modelConfig, setModelConfig, onSave }: SettingTabsProps) {
    return (
        <Tabs defaultValue="account" className="w-[400px]">
  <TabsList>
    <TabsTrigger value="account">Account</TabsTrigger>
    <TabsTrigger value="password">Password</TabsTrigger>
  </TabsList>
  <TabsContent value="account">
    <ModelSettingsModal
// showModelSettings={showModelSettings}
setShowModelSettings={setShowModelSettings}
modelConfig={modelConfig}
setModelConfig={setModelConfig}
onSave={onSave}
/>
  </TabsContent>
  <TabsContent value="password">Change your password here.</TabsContent>
</Tabs>
    )
}


