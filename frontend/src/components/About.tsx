import React from "react";
import { invoke } from '@tauri-apps/api/core';

export function About() {
    const handleContactClick = async () => {
        try {
            await invoke('open_external_url', { url: 'https://meetily.zackriya.com/#pricing' });
        } catch (error) {
            console.error('Failed to open link:', error);
        }
    };

    return (
        <div className="space-y-6 p-2">
            {/* Version Section */}
            <div className="text-center">
                <h2 className="text-2xl font-bold text-gray-800 mb-2">Meetily</h2>
                <p className="text-sm text-gray-500">Version 0.0.5 - Pre Release</p>
            </div>

            {/* Description Section */}
            <div className="space-y-3">
                <p className="text-sm text-gray-600 leading-relaxed ">
                    An AI-powered meeting assistant that captures live meeting audio, transcribes it in real-time, 
                    and generates summaries while ensuring user privacy. Perfect for teams who want to focus on 
                    discussions while automatically capturing and organizing meeting content.
                </p>
            </div>

            {/* Why Meetily Section */}
            <div className="space-y-3">
                <h3 className="text-lg font-semibold text-gray-700">Why Meetily?</h3>
                <p className="text-sm text-gray-600 leading-relaxed">
                    While there are many meeting transcription tools available, this solution stands out by offering:
                </p>
                <ul className="text-sm text-gray-600 space-y-2 ml-4">
                    <li><strong>Privacy First:</strong> All processing happens locally on your device</li>
                    <li><strong>Cost Effective:</strong> Uses open-source AI models instead of expensive APIs</li>
                    <li><strong>Flexible:</strong> Works offline, supports multiple meeting platforms</li>
                    <li><strong>Customizable:</strong> Self-host and modify for your specific needs</li>
                    <li><strong>Intelligent:</strong> Built-in knowledge graph for semantic search across meetings</li>
                </ul>
            </div>

            {/* Made By Section */}
            <div className="space-y-2">
                <p className="text-sm text-gray-600">
                    <span className="text-lg font-semibold text-gray-700">Made by </span>
                    <span className="font-medium">Zackriya Solutions</span>
                </p>
                <p className="text-xs text-gray-500">
                    Innovative AI solutions for modern productivity
                </p>
            </div>

            {/* Contact Section */}
            <div className="space-y-2">
                <button 
                    onClick={handleContactClick}
                    className="text-blue-600 hover:text-blue-800 text-lg font-semibold transition-colors duration-200 bg-transparent border-none p-0 cursor-pointer"
                >
                    Reach out to us
                </button>
            </div>
        </div>
    )
}