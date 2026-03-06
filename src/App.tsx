import { useEffect } from "react";
import { useTauriEvents } from "@/hooks/useTauriEvents";
import { useCommands } from "@/hooks/useCommands";
import { Header } from "@/components/layout/Header";
import { Sidebar } from "@/components/layout/Sidebar";
import { ErrorBanner } from "@/components/common/ErrorBanner";
import { UpdateBanner } from "@/components/common/UpdateBanner";
import { LastScanResult } from "@/components/scan/LastScanResult";
import { CodeBuffer } from "@/components/scan/CodeBuffer";
import { useAppStore } from "@/store/useAppStore";

export default function App() {
  useTauriEvents();
  const { listPdfs, listPrinters, getSettings } = useCommands();
  const currentError = useAppStore((s) => s.currentError);
  const setError = useAppStore((s) => s.setError);
  const appMode = useAppStore((s) => s.appMode);

  useEffect(() => {
    listPdfs();
    listPrinters();
    getSettings();
  }, [listPdfs, listPrinters, getSettings]);

  return (
    <div className="flex flex-col h-screen bg-background text-foreground overflow-hidden">
      <Header />
      <div className="flex flex-1 overflow-hidden">
        <main className="flex-1 flex flex-col gap-4 p-5 overflow-y-auto min-w-0">
          <UpdateBanner />
          {currentError && (
            <ErrorBanner
              message={currentError}
              onDismiss={() => setError(null)}
            />
          )}
          <LastScanResult />
          {appMode === "Buffered" && <CodeBuffer />}
        </main>
        <Sidebar />
      </div>
    </div>
  );
}
