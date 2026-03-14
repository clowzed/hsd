import { useEffect, useState } from "react";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useAppStore } from "@/store/useAppStore";

export function UpdateBanner() {
  const [ready, setReady] = useState(false);
  const addLog = useAppStore((s) => s.addUpdaterLog);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        addLog("Checking for updates...");
        const update = await check();
        if (cancelled) return;

        if (!update) {
          addLog("No update available (current version is latest)");
          return;
        }

        addLog(
          `Update found: v${update.version} (current: ${update.currentVersion}), date: ${update.date ?? "unknown"}`
        );
        if (update.body) {
          addLog(`Release notes: ${update.body}`);
        }

        addLog("Downloading and installing update...");
        await update.downloadAndInstall((event) => {
          if (event.event === "Started") {
            addLog(
              `Download started, size: ${event.data.contentLength ?? "unknown"} bytes`
            );
          } else if (event.event === "Progress") {
            addLog(
              `Download progress: ${event.data.chunkLength} bytes chunk`
            );
          } else if (event.event === "Finished") {
            addLog("Download finished");
          }
        });

        if (!cancelled) {
          addLog("Update ready, showing restart banner");
          setReady(true);
        }
      } catch (e) {
        addLog(`Update error: ${e}`);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [addLog]);

  if (!ready) return null;

  return (
    <div className="flex items-center gap-3 px-4 py-3 bg-blue-50 dark:bg-blue-950/40 border border-blue-200 dark:border-blue-800 rounded-lg text-sm">
      <RefreshCw className="w-4 h-4 text-blue-500 dark:text-blue-400 shrink-0" />
      <span className="flex-1 text-blue-700 dark:text-blue-300">
        Обновление загружено
      </span>
      <Button
        size="sm"
        variant="outline"
        className="border-blue-300 dark:border-blue-700 text-blue-700 dark:text-blue-300 hover:bg-blue-100 dark:hover:bg-blue-900/40"
        onClick={() => relaunch()}
      >
        Перезапустить
      </Button>
    </div>
  );
}
