export function getDoneTaskVisualStyle(
  isDone: boolean,
  isTodayTab: boolean
): string {
  if (!isDone) {
    return "";
  }

  return isTodayTab ? "opacity-45 saturate-75" : "opacity-60";
}
