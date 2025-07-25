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
                <h2 className="text-2xl font-bold text-gray-800 mb-2">Meetily — v0.0.5 (Pre‑release)</h2>
                <p className="text-sm text-gray-500">Real‑time notes and summaries that never leave your machine.</p>
            </div>

            {/* What makes Meetily different Section */}
            <div className="space-y-3">
                <h3 className="text-lg font-semibold text-gray-700">What makes Meetily different</h3>
                <ul className="text-sm text-gray-600 space-y-2 ml-4">
                    <li><strong>Privacy‑first</strong> All audio & AI processing stay on your own system—no cloud, no leaks.</li>
                    <li><strong>Cost‑smart</strong> Runs on open‑source models, not pricey pay‑per‑minute APIs.</li>
                    <li><strong>Works everywhere</strong> Google Meet, Zoom, Teams—online or offline.</li>
                    <li><strong>Searchable memory</strong> Instantly find any decision or action with built‑in semantic search.</li>
                    <li><strong>Yours to shape</strong> Self‑host, customise, and extend to fit the way you work.</li>
                </ul>
            </div>

            {/* Take the next step Section */}
            <div className="space-y-3">
                <h3 className="text-lg font-semibold text-gray-700">Take the next step</h3>
                <p className="text-sm text-gray-600 leading-relaxed">
                    Have a bigger idea—an AI agent, a tailored workflow, a full product?<br />
                    Let's build it together.
                </p>
            </div>

            {/* Contact Section */}
            <div className="space-y-2">
                <button 
                    onClick={handleContactClick}
                    className="text-blue-600 hover:text-blue-800 text-lg font-semibold transition-colors duration-200 bg-transparent border-none p-0 cursor-pointer"
                >
                    Talk to the Zackriya team
                </button>
            </div>
        </div>
    )
}