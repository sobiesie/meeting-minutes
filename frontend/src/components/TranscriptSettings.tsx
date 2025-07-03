import { useState, useEffect } from 'react';


export interface TranscriptModelProps {
    provider: 'localWhisper' | 'deepgram' | 'elevenLabs' | 'groq' | 'openai';
    model: string;
    apiKey?: string | null;
}

export interface TranscriptSettingsProps {
    transcriptModelConfig: TranscriptModelProps;
    setTranscriptModelConfig: (config: TranscriptModelProps) => void;
    onSave: (config: TranscriptModelProps) => void;
}

export function TranscriptSettings({ transcriptModelConfig, setTranscriptModelConfig, onSave }: TranscriptSettingsProps) {
    const [error, setError] = useState<string>('');
    const [apiKey, setApiKey] = useState<string | null>(transcriptModelConfig.apiKey || null);
    const [showApiKey, setShowApiKey] = useState<boolean>(false);
    const [isApiKeyLocked, setIsApiKeyLocked] = useState<boolean>(true);
    const [isLockButtonVibrating, setIsLockButtonVibrating] = useState<boolean>(false);

    useEffect(() => {
        const fetchTranscriptSettings = async () => {
            const response = await fetch('http://localhost:5167/get-transcript-settings');
            const data = await response.json();
            if (data.provider !== null) {
                setTranscriptModelConfig(data);
                setApiKey(data.apiKey || null);
            }
        };
        fetchTranscriptSettings();
    }, []);

    useEffect(() => {
        if (transcriptModelConfig.provider === 'localWhisper') {
            setApiKey(null);
        }
    }, [transcriptModelConfig.provider]);

    return (
        <div>
            <h1>Transcript Settings</h1>
        </div>
    );
}








