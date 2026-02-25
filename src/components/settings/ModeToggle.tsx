import { useAppStore } from "@/store/useAppStore";
import { useCommands } from "@/hooks/useCommands";
import { Button } from "@/components/ui/button";
import { Layers, Zap } from "lucide-react";

export function ModeToggle() {
  const appMode = useAppStore((s) => s.appMode);
  const { setMode } = useCommands();

  return (
    <div className="flex rounded-md border border-input overflow-hidden">
      <Button
        variant="ghost"
        size="sm"
        className={`rounded-none gap-1.5 h-8 px-3 text-xs ${
          appMode === "Buffered"
            ? "bg-primary text-primary-foreground hover:bg-primary hover:text-primary-foreground"
            : "text-muted-foreground hover:text-foreground"
        }`}
        onClick={() => setMode("Buffered")}
      >
        <Layers className="w-3.5 h-3.5" />
        Буфер
      </Button>
      <Button
        variant="ghost"
        size="sm"
        className={`rounded-none gap-1.5 h-8 px-3 text-xs border-l border-input ${
          appMode === "Instant"
            ? "bg-primary text-primary-foreground hover:bg-primary hover:text-primary-foreground"
            : "text-muted-foreground hover:text-foreground"
        }`}
        onClick={() => setMode("Instant")}
      >
        <Zap className="w-3.5 h-3.5" />
        Печать
      </Button>
    </div>
  );
}
