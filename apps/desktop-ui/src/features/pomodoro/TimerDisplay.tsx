interface TimerDisplayProps {
  time: string;
  status: string;
  progress: number;
  isWorkMode: boolean;
}

export function TimerDisplay({ time, status, progress, isWorkMode }: TimerDisplayProps) {
  const radius = 75;
  const stroke = 6;
  const normalizedProgress = 100 - progress;
  const circumference = 2 * Math.PI * radius;
  const strokeDashoffset = circumference - (normalizedProgress / 100) * circumference;

  return (
    <div className="relative flex flex-col items-center mb-2">
      {/* Circular progress ring */}
      <div className="relative">
        <svg
          width={radius * 2 + stroke * 2}
          height={radius * 2 + stroke * 2}
          className="transform -rotate-90"
        >
          {/* Background circle */}
          <circle
            cx={radius + stroke}
            cy={radius + stroke}
            r={radius}
            fill="none"
            stroke="currentColor"
            strokeWidth={stroke}
            className="text-space-overlay"
          />
          {/* Progress circle */}
          <circle
            cx={radius + stroke}
            cy={radius + stroke}
            r={radius}
            fill="none"
            stroke="currentColor"
            strokeWidth={stroke}
            strokeLinecap="round"
            strokeDasharray={circumference}
            strokeDashoffset={strokeDashoffset}
            className={isWorkMode ? "text-pomodoro-focus" : "text-pomodoro-rest"}
            style={{
              transition: "stroke-dashoffset 1s ease-in-out",
            }}
          />
        </svg>
        
        {/* Time display in center */}
        <div className="absolute inset-0 flex flex-col items-center justify-center">
          <span className={`font-mono text-4xl font-bold ${
            isWorkMode ? "text-pomodoro-focus" : "text-pomodoro-rest"
          }`}>
            {time}
          </span>
          <span className="text-text-secondary text-sm mt-1">{status}</span>
        </div>
      </div>
    </div>
  );
}
