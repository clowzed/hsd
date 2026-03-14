import { useCallback, useRef, useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { useAppStore } from "@/store/useAppStore";
import { useCommands } from "@/hooks/useCommands";
import { Settings, FolderOpen } from "lucide-react";
import { DevSettings } from "./DevSettings";

type SettingsTab = "main" | "dev";

export function BarcodeSettings() {
  const [open, setOpen] = useState(false);
  const [tab, setTab] = useState<SettingsTab>("main");

  const devMode = useAppStore((s) => s.devMode);
  const setDevMode = useAppStore((s) => s.setDevMode);

  const barcodeEnabled = useAppStore((s) => s.barcodeEnabled);
  const barcodeCopies = useAppStore((s) => s.barcodeCopies);
  const barcodeActivePreset = useAppStore((s) => s.barcodeActivePreset);
  const barcodePresets = useAppStore((s) => s.barcodePresets);
  const duplicateDetectionBuffered = useAppStore(
    (s) => s.duplicateDetectionBuffered
  );
  const duplicateDetectionInstant = useAppStore(
    (s) => s.duplicateDetectionInstant
  );

  const {
    setBarcodeSettings,
    setBarcodePresetDirectory,
    selectDirectory,
    setDuplicateDetection,
    clearScanHistory,
  } = useCommands();

  // Triple-click detection for dev mode
  const clickTimesRef = useRef<number[]>([]);
  const handleSettingsClick = useCallback(() => {
    const now = Date.now();
    clickTimesRef.current.push(now);
    // Keep only clicks within the last 800ms
    clickTimesRef.current = clickTimesRef.current.filter(
      (t) => now - t < 800
    );
    if (clickTimesRef.current.length >= 3) {
      clickTimesRef.current = [];
      setDevMode(!devMode);
    }
  }, [devMode, setDevMode]);

  const handleToggle = (checked: boolean) => {
    setBarcodeSettings(checked, barcodeCopies, barcodeActivePreset);
  };

  const handlePresetSelect = (presetName: string) => {
    const preset = barcodePresets.find((p) => p.name === presetName);
    const copies = preset?.default_copies ?? 1;
    setBarcodeSettings(barcodeEnabled, copies, presetName);
  };

  const handleCopiesChange = (value: string) => {
    const n = Math.max(1, parseInt(value) || 1);
    setBarcodeSettings(barcodeEnabled, n, barcodeActivePreset);
  };

  const handleBrowse = async (presetName: string) => {
    const dir = await selectDirectory();
    if (dir) {
      setBarcodePresetDirectory(presetName, dir);
    }
  };

  const handleOpenChange = (v: boolean) => {
    setOpen(v);
    if (!v) setTab("main");
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8 text-muted-foreground hover:text-foreground relative"
        >
          <Settings className="w-4 h-4" />
          {barcodeEnabled && (
            <span className="absolute -top-0.5 -right-0.5 h-2 w-2 rounded-full bg-primary" />
          )}
        </Button>
      </DialogTrigger>
      <DialogContent className={tab === "dev" ? "sm:max-w-lg" : "sm:max-w-md"}>
        <DialogHeader>
          <DialogTitle
            className="cursor-default select-none"
            onClick={handleSettingsClick}
          >
            Настройки
          </DialogTitle>
          <DialogDescription>
            {tab === "main"
              ? "Штрихкоды маркетплейсов и контроль дубликатов"
              : "Логи и отладка обновлений"}
          </DialogDescription>
        </DialogHeader>

        {/* Tab switcher (only when dev mode is active) */}
        {devMode && (
          <div className="flex gap-1 bg-muted rounded-lg p-1">
            <button
              className={`flex-1 text-xs py-1.5 px-3 rounded-md transition-colors ${
                tab === "main"
                  ? "bg-background shadow-sm font-medium"
                  : "text-muted-foreground hover:text-foreground"
              }`}
              onClick={() => setTab("main")}
            >
              Основные
            </button>
            <button
              className={`flex-1 text-xs py-1.5 px-3 rounded-md transition-colors ${
                tab === "dev"
                  ? "bg-background shadow-sm font-medium"
                  : "text-muted-foreground hover:text-foreground"
              }`}
              onClick={() => setTab("dev")}
            >
              Dev
            </button>
          </div>
        )}

        {tab === "main" ? (
          <div className="space-y-5">
            {/* --- Duplicate detection section --- */}
            <div className="space-y-3">
              <Label className="text-sm font-medium">
                Контроль дубликатов
              </Label>
              <div className="flex items-center justify-between">
                <Label
                  htmlFor="dup-buffered"
                  className="text-sm text-muted-foreground"
                >
                  В режиме буфера
                </Label>
                <Switch
                  id="dup-buffered"
                  checked={duplicateDetectionBuffered}
                  onCheckedChange={(checked) =>
                    setDuplicateDetection(checked, duplicateDetectionInstant)
                  }
                />
              </div>
              <div className="flex items-center justify-between">
                <Label
                  htmlFor="dup-instant"
                  className="text-sm text-muted-foreground"
                >
                  В режиме печати
                </Label>
                <Switch
                  id="dup-instant"
                  checked={duplicateDetectionInstant}
                  onCheckedChange={(checked) =>
                    setDuplicateDetection(duplicateDetectionBuffered, checked)
                  }
                />
              </div>
              <Button
                variant="outline"
                size="sm"
                onClick={clearScanHistory}
                className="w-full"
              >
                Сбросить историю сканирований
              </Button>
            </div>

            <Separator />

            {/* --- Barcode printing section --- */}
            <div className="flex items-center justify-between">
              <Label
                htmlFor="barcode-toggle"
                className="text-sm font-medium"
              >
                Печатать штрихкоды
              </Label>
              <Switch
                id="barcode-toggle"
                checked={barcodeEnabled}
                onCheckedChange={handleToggle}
              />
            </div>

            {/* Preset selector */}
            <div className="space-y-2">
              <Label className="text-sm text-muted-foreground">Пресет</Label>
              <div className="flex gap-2">
                {barcodePresets.map((preset) => (
                  <Button
                    key={preset.name}
                    variant={
                      barcodeActivePreset === preset.name
                        ? "default"
                        : "outline"
                    }
                    size="sm"
                    className="flex-1"
                    disabled={!barcodeEnabled}
                    onClick={() => handlePresetSelect(preset.name)}
                  >
                    {preset.name}
                    <span className="ml-1.5 text-xs opacity-70">
                      x{preset.default_copies}
                    </span>
                  </Button>
                ))}
              </div>
            </div>

            {/* Copies input */}
            <div className="space-y-2">
              <Label
                htmlFor="barcode-copies"
                className="text-sm text-muted-foreground"
              >
                Количество копий
              </Label>
              <Input
                id="barcode-copies"
                type="number"
                min={1}
                max={10}
                value={barcodeCopies}
                onChange={(e) => handleCopiesChange(e.target.value)}
                disabled={!barcodeEnabled}
                className="w-24"
              />
            </div>

            {/* Preset directories */}
            <div className="space-y-3">
              <Label className="text-sm text-muted-foreground">
                Папки со штрихкодами
              </Label>
              {barcodePresets.map((preset) => (
                <div key={preset.name} className="space-y-1.5">
                  <span className="text-xs font-medium text-muted-foreground">
                    {preset.name}
                  </span>
                  <div className="flex gap-2">
                    <Input
                      value={preset.directory}
                      readOnly
                      placeholder="Выберите папку..."
                      disabled={!barcodeEnabled}
                      className="flex-1 text-xs"
                    />
                    <Button
                      variant="outline"
                      size="sm"
                      disabled={!barcodeEnabled}
                      onClick={() => handleBrowse(preset.name)}
                      className="shrink-0 gap-1.5"
                    >
                      <FolderOpen className="w-3.5 h-3.5" />
                      Выбрать
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          </div>
        ) : (
          <DevSettings />
        )}
      </DialogContent>
    </Dialog>
  );
}
