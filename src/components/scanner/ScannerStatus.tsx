import { cn } from "@/lib/utils";
import { useAppStore } from "@/store/useAppStore";

const statusConfig = {
  Connected: {
    dot: "bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.6)]",
    text: "text-emerald-600",
    label: "Подключен",
  },
  Connecting: {
    dot: "bg-amber-400 animate-pulse",
    text: "text-amber-600",
    label: "Подключение...",
  },
  Disconnected: {
    dot: "bg-red-500",
    text: "text-red-600",
    label: "Отключен",
  },
  Error: {
    dot: "bg-red-500",
    text: "text-red-600",
    label: "Ошибка",
  },
};

export function ScannerStatusIndicator() {
  const status = useAppStore((s) => s.scannerStatus);
  const c = statusConfig[status.type];

  return (
    <div
      className="flex items-center gap-2 shrink-0"
      title={
        status.type === "Error" ? `${c.label}: ${status.message}` : c.label
      }
    >
      <div className={cn("w-2.5 h-2.5 rounded-full shrink-0", c.dot)} />
      <span className={cn("text-xs font-medium whitespace-nowrap", c.text)}>
        {c.label}
      </span>
    </div>
  );
}
