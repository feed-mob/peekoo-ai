import { AnimatePresence, motion } from "framer-motion";
import { Droplet, Eye, PersonStanding, Activity, type LucideIcon } from "lucide-react";
import {
  PEEK_BADGE_EXPANDED_VERTICAL_PADDING,
  PEEK_BADGE_HEIGHT,
  PEEK_BADGE_PADDING,
  PEEK_BADGE_ROW_HEIGHT,
} from "@/lib/sprite-bubble-layout";
import type { PeekBadgeItem } from "@/types/peek-badge";

const BADGE_WIDTH = 188;
const VALUE_COLUMN_WIDTH = 64;

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
  const rowHeight = compact ? PEEK_BADGE_ROW_HEIGHT : PEEK_BADGE_HEIGHT;

  if (!compact) {
    return (
      <div
        className="flex flex-col justify-center gap-0.5"
        style={{ height: rowHeight }}
      >
        <div className="flex min-w-0 items-center gap-1.5 overflow-hidden">
          <BadgeIcon name={item.icon} className="shrink-0 text-glow-cyan/70" />
          <span className="truncate whitespace-nowrap text-[11px] font-medium leading-none text-text-primary/90">
            {item.label}
          </span>
        </div>
        <span className="truncate pl-[18px] text-[11px] leading-none tabular-nums text-glow-cyan/80">
          {item.value}
        </span>
      </div>
    );
  }

  return (
    <div
      className="grid items-center gap-x-2"
      style={{
        height: rowHeight,
        gridTemplateColumns: `minmax(0, 1fr) ${VALUE_COLUMN_WIDTH}px`,
      }}
    >
      <div className="flex min-w-0 items-center gap-1.5 overflow-hidden">
        <BadgeIcon name={item.icon} className="shrink-0 text-glow-cyan/70" />
        <span className="truncate whitespace-nowrap text-[11px] font-medium leading-none text-text-primary/90">
          {item.label}
        </span>
      </div>
      <span className="truncate text-right text-[11px] leading-none tabular-nums text-glow-cyan/80">
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

  const expandedHeight = items.length * PEEK_BADGE_ROW_HEIGHT + PEEK_BADGE_EXPANDED_VERTICAL_PADDING;

  return (
    <AnimatePresence mode="wait">
      {expanded ? (
        <motion.div
          key="expanded"
          initial={{ opacity: 0, y: 4, scale: 0.97 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          exit={{ opacity: 0, y: 4, scale: 0.97 }}
          transition={{ duration: 0.15, ease: "easeOut" }}
          className="pointer-events-auto absolute z-10 cursor-pointer rounded-xl border border-glass-border bg-glass/90 px-3 py-2 shadow-panel backdrop-blur-lg"
          style={{
            top: PEEK_BADGE_PADDING,
            left: "50%",
            marginLeft: -(BADGE_WIDTH / 2),
            width: BADGE_WIDTH,
            minHeight: expandedHeight,
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
          className="pointer-events-auto absolute z-10 cursor-pointer rounded-xl border border-glass-border/60 bg-glass/80 px-3 py-1 shadow-panel/50 backdrop-blur-md"
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
