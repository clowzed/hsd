import { useAppStore } from "@/store/useAppStore";
import { useCommands } from "@/hooks/useCommands";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { RefreshCw, Printer } from "lucide-react";

export function PrinterSelect() {
  const printers = useAppStore((s) => s.printers);
  const selectedPrinter = useAppStore((s) => s.selectedPrinter);
  const { setPrinter, listPrinters } = useCommands();

  return (
    <div className="flex items-center gap-1.5">
      <Printer className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
      <Select
        value={selectedPrinter ?? ""}
        onValueChange={(v) => setPrinter(v)}
      >
        <SelectTrigger className="h-8 w-[140px] text-xs">
          <SelectValue placeholder="Выберите принтер" />
        </SelectTrigger>
        <SelectContent>
          {printers.map((p) => (
            <SelectItem key={p.name} value={p.name} className="text-xs">
              {p.name}
              {p.is_default && (
                <span className="ml-1 text-muted-foreground">(по умолч.)</span>
              )}
            </SelectItem>
          ))}
          {printers.length === 0 && (
            <div className="px-2 py-1.5 text-xs text-muted-foreground">
              Принтеры не найдены
            </div>
          )}
        </SelectContent>
      </Select>
      <Button
        variant="ghost"
        size="icon"
        className="h-8 w-8 text-muted-foreground hover:text-foreground shrink-0"
        onClick={() => listPrinters()}
      >
        <RefreshCw className="w-3.5 h-3.5" />
      </Button>
    </div>
  );
}
