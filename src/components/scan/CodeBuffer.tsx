import { useAppStore } from "@/store/useAppStore";
import { useCommands } from "@/hooks/useCommands";
import { Card, CardContent } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { Package, X, FileCheck, FileX } from "lucide-react";

export function CodeBuffer() {
  const codes = useAppStore((s) => s.scannedCodes);
  const { removeCode } = useCommands();

  return (
    <div className="space-y-2 flex-1 min-h-0 flex flex-col">
      <h2 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground px-0.5">
        Буфер кодов ({codes.length})
      </h2>

      <Card className="flex-1 flex flex-col min-h-0">
        {codes.length === 0 ? (
          <CardContent className="flex flex-col items-center justify-center flex-1 min-h-[100px] gap-2 text-muted-foreground">
            <Package className="w-6 h-6 opacity-30" />
            <span className="text-sm">Отсканированные коды появятся здесь</span>
          </CardContent>
        ) : (
          <ScrollArea className="flex-1 max-h-[280px]">
            <div className="divide-y divide-border">
              {codes.map((code, i) => (
                <div
                  key={i}
                  className="flex items-center gap-3 px-4 py-2.5 text-sm hover:bg-muted/40 transition-colors group"
                >
                  <span className="text-xs text-muted-foreground w-5 shrink-0 text-right tabular-nums">
                    {i + 1}
                  </span>
                  <span className="font-mono text-xs font-medium shrink-0 text-foreground">
                    {code.gtin}
                  </span>
                  {code.vendor_code && (
                    <span className="text-xs text-muted-foreground shrink-0 flex items-center gap-1">
                      {code.vendor_code}
                      {code.barcode_exists ? (
                        <FileCheck className="w-3 h-3 text-emerald-500" />
                      ) : (
                        <FileX className="w-3 h-3 text-red-400" />
                      )}
                    </span>
                  )}
                  <span className="text-muted-foreground truncate text-xs flex-1">
                    {code.product_name}
                  </span>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity shrink-0 text-muted-foreground hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-950/40"
                    onClick={() => removeCode(i)}
                  >
                    <X className="w-3.5 h-3.5" />
                  </Button>
                </div>
              ))}
            </div>
          </ScrollArea>
        )}
      </Card>
    </div>
  );
}
