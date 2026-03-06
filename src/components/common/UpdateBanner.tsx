import { useEffect, useState } from "react";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";

export function UpdateBanner() {
  const [ready, setReady] = useState(false);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        const update = await check();
        if (cancelled || !update) return;

        await update.downloadAndInstall();
        if (!cancelled) setReady(true);
      } catch {
        // silently ignore update errors
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

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
