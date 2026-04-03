import { useState } from "react";
import { RefreshCw } from "lucide-react";
import { useTaskActivity } from "../hooks/use-task-activity";
import { ActivityFeedItem } from "./ActivityFeedItem";
import { LoadingSpinner } from "./LoadingSpinner";
import { formatSyncStatus } from "../utils/task-sync";
import { useTranslation } from "react-i18next";

interface TaskActivitySectionProps {
  taskId: string;
}

export function TaskActivitySection({ taskId }: TaskActivitySectionProps) {
  const { t } = useTranslation();
  const { events, isLoading, isRefreshing, lastSyncedAt, reload, addComment, deleteEvent } = useTaskActivity(taskId);
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
      <div className="flex items-start justify-between mb-3 gap-3">
        <div>
          <h3 className="text-xs font-semibold text-text-muted uppercase tracking-wider">
            {t("tasks.mainTab.activity")}
          </h3>
          <div className="mt-1 flex items-center gap-1.5 text-[10px] text-text-muted">
            <RefreshCw size={10} className={isRefreshing ? "animate-spin" : ""} />
            <span>{formatSyncStatus(isRefreshing, lastSyncedAt, Date.now(), t)}</span>
          </div>
        </div>
        <button
          onClick={reload}
          disabled={isLoading || isRefreshing}
          className="text-[10px] text-text-muted hover:text-text-primary transition-colors disabled:opacity-50"
        >
          {t("common.refresh")}
        </button>
      </div>

      {/* Comment input */}
      <form onSubmit={handleSubmitComment} className="mb-4">
        <div className="flex gap-2">
          <textarea
            value={commentText}
            onChange={(e) => setCommentText(e.target.value)}
            placeholder={t("tasks.activity.addCommentPlaceholder")}
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
            {isSubmitting ? t("tasks.activity.adding") : t("tasks.activity.addComment")}
          </button>
        </div>
      </form>

      {isLoading ? (
        <div className="flex items-center justify-center py-6">
          <LoadingSpinner size="sm" />
        </div>
      ) : events.length === 0 ? (
        <p className="text-xs text-text-muted py-2">{t("tasks.activity.noActivity")}</p>
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
