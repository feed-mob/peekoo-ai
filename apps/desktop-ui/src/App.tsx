import React, { useState, useEffect } from 'react';
import './App.css';
import Chat from './components/Chat';
import Tasks from './components/Tasks';
import Pomodoro from './components/Pomodoro';
import { invoke } from '@tauri-apps/api/core';

type Tab = 'chat' | 'tasks' | 'pomodoro';

interface PetState {
  mood: string;
  message: string;
  animation: string;
}

const MOOD_ICONS: Record<string, string> = {
  happy: '😊',
  excited: '🤩',
  thinking: '🤔',
  sad: '😢',
  tired: '😴',
  surprised: '😲',
  idle: '😌',
};

const ANIMATION_CLASSES: Record<string, string> = {
  bounce: 'bounce',
  bounceFast: 'bounce-fast',
  pulse: 'pulse',
  pulseFast: 'pulse-fast',
  shake: 'shake',
  sway: 'sway',
  idle: '',
};

function App() {
  const [activeTab, setActiveTab] = useState<Tab>('chat');
  const [petState, setPetState] = useState<PetState>({
    mood: 'happy',
    message: 'Welcome to Peekoo! Your AI desktop pet is ready to help you!',
    animation: 'bounce',
  });

  useEffect(() => {
    // Fetch initial pet state from Tauri backend
    invoke<PetState>('get_pet_state')
      .then((state) => {
        setPetState(state);
      })
      .catch((error) => {
        console.error('Failed to fetch pet state:', error);
      });
  }, []);

  const getPetIcon = () => {
    return MOOD_ICONS[petState.mood] || MOOD_ICONS.idle;
  };

  const getAnimationClass = () => {
    return ANIMATION_CLASSES[petState.animation] || '';
  };

  return (
    <div className="app">
      {/* Pet Display */}
      <div className="pet-container">
        <div className={`pet-avatar ${getAnimationClass()}`}>
          {getPetIcon()}
        </div>
        <div className="pet-message">{petState.message}</div>
      </div>

      {/* Tabs */}
      <div className="tabs">
        <button
          className={activeTab === 'chat' ? 'active' : ''}
          onClick={() => setActiveTab('chat')}
        >
          💬 Chat
        </button>
        <button
          className={activeTab === 'tasks' ? 'active' : ''}
          onClick={() => setActiveTab('tasks')}
        >
          ✅ Tasks
        </button>
        <button
          className={activeTab === 'pomodoro' ? 'active' : ''}
          onClick={() => setActiveTab('pomodoro')}
        >
          🍅 Pomodoro
        </button>
      </div>

      {/* Content */}
      <div className="content">
        {activeTab === 'chat' && <Chat />}
        {activeTab === 'tasks' && <Tasks />}
        {activeTab === 'pomodoro' && <Pomodoro />}
      </div>
    </div>
  );
}

export default App;
