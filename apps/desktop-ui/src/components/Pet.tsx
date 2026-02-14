import React from 'react';

interface PetState {
  mood: 'happy' | 'sad' | 'excited' | 'thinking' | 'idle' | string;
  message: string;
  animation: string;
}

const MOOD_ICONS: Record<string, string> = {
  happy: '😊',
  sad: '😢',
  excited: '🤩',
  thinking: '🤔',
  idle: '😌',
};

const ANIMATION_CLASSES: Record<string, string> = {
  bounce: 'animate-bounce',
  bounceFast: 'animate-bounce',
  pulse: 'animate-pulse',
  pulseFast: 'animate-pulse',
  shake: '',
  sway: '',
  idle: '',
};

export default function Pet({ state }: { state?: PetState }) {
  const petState: PetState = state || {
    mood: 'happy',
    message: 'Welcome! Your AI desktop pet is ready to help you!',
    animation: 'bounce',
  };

  const getPetIcon = () => MOOD_ICONS[petState.mood] || MOOD_ICONS.idle;

  const getAnimationClass = () => ANIMATION_CLASSES[petState.animation] || '';

  return (
    <div className="flex flex-col items-center justify-center p-8 bg-white/10 backdrop-blur-md rounded-2xl shadow-lg">
      <div className={`text-8xl mb-4 ${getAnimationClass()}`}>
        {getPetIcon()}
      </div>
      <div className="bg-white px-6 py-3 rounded-full shadow-md max-w-md text-center">
        <p className="text-gray-800 font-medium">{petState.message}</p>
      </div>
    </div>
  );
}
