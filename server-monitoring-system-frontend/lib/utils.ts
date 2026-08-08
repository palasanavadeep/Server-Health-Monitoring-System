import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function formatBytes(bytes: number, decimals = 2) {
  if (bytes === 0) return '0 Bytes';
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
}

/**
 * Returns dynamic Tailwind color text classes based on latency value thresholds
 * - < 300ms: Emerald green
 * - 300ms - 700ms: White/Default foreground
 * - > 700ms and < 1000ms: Orange
 * - >= 1000ms: Rose red
 */
export function getLatencyColorClass(latencyMs: number): string {
  if (latencyMs < 300) {
    return "text-emerald-400";
  }
  if (latencyMs <= 700) {
    return "text-foreground";
  }
  if (latencyMs < 1000) {
    return "text-orange-400";
  }
  return "text-rose-400 font-bold  animate-pulse";
}
