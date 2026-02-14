import React, { useState } from 'react';

interface Task {
  id: string;
  title: string;
  completed: boolean;
  priority: 'low' | 'medium' | 'high';
}

export default function Tasks() {
  const [tasks, setTasks] = useState<Task[]>([
    { id: '1', title: 'Complete project documentation', completed: false, priority: 'high' },
    { id: '2', title: 'Review pull requests', completed: true, priority: 'medium' },
    { id: '3', title: 'Update dependencies', completed: false, priority: 'low' },
  ]);
  const [newTask, setNewTask] = useState('');
  const [priority, setPriority] = useState<'low' | 'medium' | 'high'>('medium');

  const handleAddTask = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newTask.trim()) return;

    const task: Task = {
      id: Date.now().toString(),
      title: newTask,
      completed: false,
      priority,
    };

    setTasks([...tasks, task]);
    setNewTask('');

    // TODO: Call Tauri backend
    // await invoke('create_task', { title: newTask, priority });
  };

  const toggleTask = (id: string) => {
    setTasks(tasks.map(task =>
      task.id === id ? { ...task, completed: !task.completed } : task
    ));
  };

  const deleteTask = (id: string) => {
    setTasks(tasks.filter(task => task.id !== id));
  };

  const getPriorityClass = (priority: string) => {
    switch (priority) {
      case 'high': return 'high';
      case 'medium': return 'medium';
      case 'low': return 'low';
      default: return 'medium';
    }
  };

  return (
    <div className="tasks-section">
      <form className="add-task" onSubmit={handleAddTask}>
        <input
          type="text"
          value={newTask}
          onChange={(e) => setNewTask(e.target.value)}
          placeholder="Add a new task..."
        />
        <select
          value={priority}
          onChange={(e) => setPriority(e.target.value as 'low' | 'medium' | 'high')}
        >
          <option value="low">Low</option>
          <option value="medium">Medium</option>
          <option value="high">High</option>
        </select>
        <button type="submit">Add Task</button>
      </form>

      <div className="task-list">
        {tasks.map((task) => (
          <div key={task.id} className={`task-item ${task.completed ? 'done' : ''}`}>
            <input
              type="checkbox"
              checked={task.completed}
              onChange={() => toggleTask(task.id)}
            />
            <span className="task-title">{task.title}</span>
            
            <span className={`task-priority ${getPriorityClass(task.priority)}`}>
              {task.priority}
            </span>
            
            <button onClick={() => deleteTask(task.id)} title="Delete task">
              🗑️
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
