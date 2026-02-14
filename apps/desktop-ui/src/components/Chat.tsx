import React, { useState, useRef, useEffect } from 'react';

interface Message {
  id: string;
  role: 'user' | 'pet';
  text: string;
}

export default function Chat() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [isTyping, setIsTyping] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!input.trim()) return;

    // Add user message
    const userMessage: Message = {
      id: Date.now().toString(),
      role: 'user',
      text: input,
    };
    setMessages(prev => [...prev, userMessage]);
    setInput('');
    setIsTyping(true);

    // TODO: Connect to Tauri backend
    // const response = await invoke<string>('send_message', { message: input });
    
    // Simulate response for now
    setTimeout(() => {
      const petMessage: Message = {
        id: (Date.now() + 1).toString(),
        role: 'pet',
        text: 'I received your message! AI integration coming soon...',
      };
      setMessages(prev => [...prev, petMessage]);
      setIsTyping(false);
    }, 1000);
  };

  return (
    <div className="chat-section">
      <div className="chat-history">
        {messages.length === 0 ? (
          <div className="empty-state">
            Start chatting with your Peekoo pet!
          </div>
        ) : (
          messages.map((msg) => (
            <div key={msg.id} className={`chat-message ${msg.role}`}>
              <span className="avatar">{msg.role === 'user' ? '👤' : '🐱'}</span>
              <div className="text">{msg.text}</div>
            </div>
          ))
        )}
        {isTyping && (
          <div className="chat-message pet">
            <span className="avatar">🐱</span>
            <div className="text">Typing...</div>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      <form className="chat-input" onSubmit={handleSubmit}>
        <input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="Type a message..."
          disabled={isTyping}
        />
        <button type="submit" disabled={isTyping || !input.trim()}>
          Send
        </button>
      </form>
    </div>
  );
}
