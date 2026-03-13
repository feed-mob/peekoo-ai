import { AnimatePresence, motion } from "framer-motion";
import { Droplet, Eye, PersonStanding, Activity, type LucideIcon } from "lucide-react";
import {
  PEEK_BADGE_HEIGHT,
  PEEK_BADGE_PADDING,
  PEEK_BADGE_ROW_HEIGHT,
} from "@/lib/sprite-bubble-layout";
import type { PeekBadgeItem } from "@/types/peek-badge";

const BADGE_WIDTH = 180;

const ICON_MAP: Record<string, LucideIcon> = {
  droplet: Droplet,
  eye: Eye,
  "person-standing": PersonStanding,
  activity: Activity,
};

function BadgeIcon({ name, className }: { name?: string; className?: string }) {
  const Icon = name ? ICON_MAP[name] : undefined;
  if (!Icon) return null;
  return <Icon size={12} className={className} />;
}

interface BadgeRowProps {
  item: PeekBadgeItem;
  compact?: boolean;
}

function BadgeRow({ item, compact }: BadgeRowProps) {
  return (
    <div
      className="flex items-center justify-between gap-1.5"
      style={{ height: compact ? PEEK_BADGE_ROW_HEIGHT : PEEK_BADGE_HEIGHT }}
    >
      <div className="flex items-center gap-1.5 min-w-0">
        <BadgeIcon name={item.icon} className="shrink-0 text-glow-cyan/70" />
        <span className="truncate text-[11px] font-medium text-text-primary/90">
          {item.label}
        </span>
      </div>
      <span className="shrink-0 text-[11px] tabular-nums text-glow-cyan/80">
        {item.value}
      </span>
    </div>
  );
}

interface SpritePeekBadgeProps {
  items: PeekBadgeItem[];
  currentItem: PeekBadgeItem | null;
  expanded: boolean;
  visible: boolean;
  onToggle: () => void;
}

export function SpritePeekBadge({
  items,
  currentItem,
  expanded,
  visible,
  onToggle,
}: SpritePeekBadgeProps) {
  if (!visible || items.length === 0 || !currentItem) return null;

  const expandedHeight = items.length * PEEK_BADGE_ROW_HEIGHT + PEEK_BADGE_PADDING;

  return (
    <AnimatePresence mode="wait">
      {expanded ? (
        <motion.div
          key="expanded"
          initial={{ opacity: 0, y: 4, scale: 0.97 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          exit={{ opacity: 0, y: 4, scale: 0.97 }}
          transition={{ duration: 0.15, ease: "easeOut" }}
          className="pointer-events-auto absolute z-10 cursor-pointer rounded-xl border border-glass-border bg-glass/90 px-3 py-1.5 shadow-panel backdrop-blur-lg"
          style={{
            top: PEEK_BADGE_PADDING,
            left: "50%",
            marginLeft: -(BADGE_WIDTH / 2),
            width: BADGE_WIDTH,
            maxHeight: expandedHeight,
          }}
          onClick={onToggle}
        >
          {items.map((item) => (
            <BadgeRow key={item.label} item={item} compact />
          ))}
        </motion.div>
      ) : (
        <motion.div
          key={`collapsed-${currentItem.label}`}
          initial={{ opacity: 0, y: 4 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -4 }}
          transition={{ duration: 0.2, ease: "easeOut" }}
          className="pointer-events-auto absolute z-10 cursor-pointer rounded-xl border border-glass-border/60 bg-glass/80 px-3 shadow-panel/50 backdrop-blur-md"
          style={{
            top: PEEK_BADGE_PADDING,
            left: "50%",
            marginLeft: -(BADGE_WIDTH / 2),
            width: BADGE_WIDTH,
            height: PEEK_BADGE_HEIGHT,
          }}
          onClick={onToggle}
        >
          <BadgeRow item={currentItem} />
        </motion.div>
      )}
    </AnimatePresence>
  );
}
