import { ScannerStatusIndicator } from "@/components/scanner/ScannerStatus";
import { ModeToggle } from "@/components/settings/ModeToggle";
import { PrinterSelect } from "@/components/settings/PrinterSelect";
import { Button } from "@/components/ui/button";
import { useAppStore } from "@/store/useAppStore";
import { useThemeStore } from "@/store/useThemeStore";
import { useCommands } from "@/hooks/useCommands";
import { Trash2, Loader2, Sun, Moon, Printer } from "lucide-react";

export function Header() {
  const scannedCodes = useAppStore((s) => s.scannedCodes);
  const appMode = useAppStore((s) => s.appMode);
  const isPrinting = useAppStore((s) => s.isPrinting);
  const currentBufferedPdf = useAppStore((s) => s.currentBufferedPdf);
  const { clearBuffer, printPdf } = useCommands();
  const canAct = scannedCodes.length > 0;
  const theme = useThemeStore((s) => s.theme);
  const toggleTheme = useThemeStore((s) => s.toggleTheme);

  const handlePrint = () => {
    if (currentBufferedPdf) {
      printPdf(currentBufferedPdf.path);
    }
  };

  return (
    <header className="flex items-center justify-between px-5 py-3 bg-card border-b shadow-[0_1px_3px_rgba(0,0,0,0.04)] dark:shadow-none shrink-0 gap-3">
      <ScannerStatusIndicator />

      <div className="flex items-center gap-3 flex-1 justify-center">
        <ModeToggle />
        <PrinterSelect />
      </div>

      <div className="flex items-center gap-2">
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8 text-muted-foreground hover:text-foreground"
          onClick={toggleTheme}
        >
          {theme === "light" ? (
            <Moon className="w-4 h-4" />
          ) : (
            <Sun className="w-4 h-4" />
          )}
        </Button>

        {appMode === "Buffered" && (
          <>
            <Button
              variant="outline"
              size="sm"
              disabled={!canAct}
              onClick={clearBuffer}
              className="gap-1.5"
            >
              <Trash2 className="w-3.5 h-3.5" />
              Очистить ({scannedCodes.length})
            </Button>
            <Button
              size="sm"
              disabled={!currentBufferedPdf || isPrinting}
              onClick={handlePrint}
              className="gap-1.5"
            >
              {isPrinting ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
              ) : (
                <Printer className="w-3.5 h-3.5" />
              )}
              {isPrinting ? "Печать..." : "Печатать"}
            </Button>
          </>
        )}
      </div>
    </header>
  );
}
