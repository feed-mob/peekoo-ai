import { useState } from "react";
import { useTaskActivity } from "../hooks/use-task-activity";
import { ActivityFeedItem } from "./ActivityFeedItem";
import { LoadingSpinner } from "./LoadingSpinner";

interface TaskActivitySectionProps {
  taskId: string;
}

export function TaskActivitySection({ taskId }: TaskActivitySectionProps) {
  const { events, isLoading, reload, addComment, deleteEvent } = useTaskActivity(taskId);
  const [commentText, setCommentText] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [deletingEventId, setDeletingEventId] = useState<string | null>(null);

  const handleSubmitComment = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!commentText.trim() || isSubmitting) return;

    setIsSubmitting(true);
    try {
      await addComment(commentText.trim());
      setCommentText("");
    } catch (err) {
      console.error("Failed to add comment:", err);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleDeleteEvent = async (eventId: string) => {
    setDeletingEventId(eventId);
    try {
      await deleteEvent(eventId);
    } catch (err) {
      console.error("Failed to delete event:", err);
    } finally {
      setDeletingEventId(null);
    }
  };

  return (
    <div className="mt-6 pt-6 border-t border-glass-border">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-xs font-semibold text-text-muted uppercase tracking-wider">
          Activity
        </h3>
        <button
          onClick={reload}
          disabled={isLoading}
          className="text-[10px] text-text-muted hover:text-text-primary transition-colors disabled:opacity-50"
        >
          Refresh
        </button>
      </div>

      {/* Comment input */}
      <form onSubmit={handleSubmitComment} className="mb-4">
        <div className="flex gap-2">
          <textarea
            value={commentText}
            onChange={(e) => setCommentText(e.target.value)}
            placeholder="Add a comment..."
            rows={2}
            disabled={isSubmitting}
            className="flex-1 px-3 py-2 text-xs bg-space-deep border border-glass-border rounded-lg resize-none focus:outline-none focus:border-glow-green/50 text-text-primary placeholder:text-text-muted disabled:opacity-50"
          />
        </div>
        <div className="flex justify-end mt-2">
          <button
            type="submit"
            disabled={!commentText.trim() || isSubmitting}
            className="px-3 py-1.5 text-xs font-medium bg-glow-green/20 text-glow-green rounded-lg hover:bg-glow-green/30 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isSubmitting ? "Adding..." : "Add Comment"}
          </button>
        </div>
      </form>

      {isLoading ? (
        <div className="flex items-center justify-center py-6">
          <LoadingSpinner size="sm" />
        </div>
      ) : events.length === 0 ? (
        <p className="text-xs text-text-muted py-2">No activity yet</p>
      ) : (
        <div className="space-y-1">
          {events.map((event) => (
            <ActivityFeedItem
              key={event.id}
              event={event}
              compact
              onDelete={handleDeleteEvent}
              isDeleting={deletingEventId === event.id}
            />
          ))}
        </div>
      )}
    </div>
  );
}