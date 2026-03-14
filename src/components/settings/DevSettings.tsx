import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useAppStore } from "@/store/useAppStore";
import { useCommands } from "@/hooks/useCommands";
import { ChevronDown, ChevronRight, RefreshCw } from "lucide-react";
import { getVersion } from "@tauri-apps/api/app";

const MAX_DISPLAY_LOGS = 100;

export function DevSettings() {
  const updaterLogs = useAppStore((s) => s.updaterLogs);
  const { getRecentLogs } = useCommands();
  const [backendLogs, setBackendLogs] = useState<string[]>([]);
  const [updaterOpen, setUpdaterOpen] = useState(true);
  const [appVersion, setAppVersion] = useState<string>("");

  useEffect(() => {
    refreshLogs();
    getVersion().then(setAppVersion).catch(() => {});
  }, []);

  const refreshLogs = async () => {
    const logs = await getRecentLogs();
    setBackendLogs(logs.slice(-MAX_DISPLAY_LOGS));
  };

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <Label className="text-sm font-medium">Dev</Label>
        {appVersion && (
          <span className="text-xs text-muted-foreground font-mono">
            v{appVersion}
          </span>
        )}
      </div>

      <Separator />

      {/* Updater section */}
      <div className="space-y-2">
        <button
          className="flex items-center gap-1.5 text-xs font-medium w-full text-left hover:text-foreground text-muted-foreground transition-colors"
          onClick={() => setUpdaterOpen(!updaterOpen)}
        >
          {updaterOpen ? (
            <ChevronDown className="w-3 h-3" />
          ) : (
            <ChevronRight className="w-3 h-3" />
          )}
          Updater
        </button>
        {updaterOpen && (
          <div className="space-y-2">
            <div className="text-[11px] text-muted-foreground space-y-0.5 bg-muted/50 rounded p-2">
              <div className="break-all">
                <span className="font-medium">Endpoint: </span>
                github.com/clowzed/hsd/releases/latest/download/latest.json
              </div>
              <div>
                <span className="font-medium">Flow: </span>
                check → downloadAndInstall → relaunch
              </div>
            </div>
            <ScrollArea className="h-32 rounded border bg-black/5 dark:bg-white/5">
              <div className="p-2 text-[11px] font-mono leading-relaxed">
                {updaterLogs.length === 0 ? (
                  <span className="text-muted-foreground">
                    Нет логов обновления
                  </span>
                ) : (
                  updaterLogs.map((log, i) => (
                    <div
                      key={i}
                      className={`break-all ${
                        log.includes("error") || log.includes("Error")
                          ? "text-red-500"
                          : log.includes("found")
                            ? "text-green-500 dark:text-green-400"
                            : "text-foreground/80"
                      }`}
                    >
                      {log}
                    </div>
                  ))
                )}
              </div>
            </ScrollArea>
          </div>
        )}
      </div>

      <Separator />

      {/* Backend logs */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label className="text-xs font-medium">
            Логи ({backendLogs.length})
          </Label>
          <Button
            variant="ghost"
            size="sm"
            onClick={refreshLogs}
            className="h-6 px-2 gap-1 text-[11px]"
          >
            <RefreshCw className="w-3 h-3" />
            Обновить
          </Button>
        </div>
        <ScrollArea className="h-48 rounded border bg-black/5 dark:bg-white/5">
          <div className="p-2 text-[11px] font-mono leading-relaxed">
            {backendLogs.length === 0 ? (
              <span className="text-muted-foreground">Нет логов</span>
            ) : (
              backendLogs.map((log, i) => (
                <div
                  key={i}
                  className={`break-all ${
                    log.includes("ERROR")
                      ? "text-red-500"
                      : log.includes("WARN")
                        ? "text-yellow-500 dark:text-yellow-400"
                        : "text-foreground/80"
                  }`}
                >
                  {log}
                </div>
              ))
            )}
          </div>
        </ScrollArea>
      </div>
    </div>
  );
}
