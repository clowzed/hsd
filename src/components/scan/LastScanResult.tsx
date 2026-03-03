import { useAppStore } from "@/store/useAppStore";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Loader2, CheckCircle2, XCircle, ScanBarcode, Printer, FileCheck, FileX } from "lucide-react";

export function LastScanResult() {
  const result = useAppStore((s) => s.lastScanResult);
  const appMode = useAppStore((s) => s.appMode);

  return (
    <div className="space-y-2">
      <h2 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground px-0.5">
        Последний скан
      </h2>

      {result.type === "None" && (
        <Card className="border-dashed border-2">
          <CardContent className="flex flex-col items-center justify-center py-10 gap-3 text-muted-foreground">
            <ScanBarcode className="w-8 h-8 opacity-40" />
            <span className="text-sm">Отсканируйте код товара</span>
          </CardContent>
        </Card>
      )}

      {result.type === "Validating" && (
        <Card className="border-amber-200 dark:border-amber-800 bg-amber-50/50 dark:bg-amber-950/30">
          <CardContent className="flex items-center justify-center gap-3 py-10 text-amber-700 dark:text-amber-400">
            <Loader2 className="w-5 h-5 animate-spin" />
            <span className="text-sm font-medium">Проверка кода...</span>
          </CardContent>
        </Card>
      )}

      {result.type === "Success" && (
        <Card className="border-emerald-200 dark:border-emerald-800 bg-emerald-50/50 dark:bg-emerald-950/30">
          <CardContent className="p-5 space-y-3">
            <div className="flex items-center gap-2.5">
              {appMode === "Instant" ? (
                <Printer className="w-5 h-5 text-emerald-600 dark:text-emerald-400" />
              ) : (
                <CheckCircle2 className="w-5 h-5 text-emerald-600 dark:text-emerald-400" />
              )}
              <Badge className="bg-emerald-600 hover:bg-emerald-600 dark:bg-emerald-700 text-white text-xs border-0">
                {appMode === "Instant" ? "НАПЕЧАТАНО" : "В ОБОРОТЕ"}
              </Badge>
            </div>
            <p className="font-semibold text-sm leading-snug text-foreground">
              {result.code.product_name}
            </p>
            <div className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-xs">
              <span className="text-muted-foreground">GTIN</span>
              <span className="font-mono font-medium">{result.code.gtin}</span>
              {result.code.vendor_code && (
                <>
                  <span className="text-muted-foreground">Артикул</span>
                  <span className="font-medium flex items-center gap-1.5">
                    {result.code.vendor_code}
                    {result.code.barcode_exists ? (
                      <FileCheck className="w-3.5 h-3.5 text-emerald-500" />
                    ) : (
                      <FileX className="w-3.5 h-3.5 text-red-400" />
                    )}
                  </span>
                </>
              )}
              {result.code.code.crypto && (
                <>
                  <span className="text-muted-foreground">Крипто</span>
                  <span className="font-mono text-[10px] break-all">{result.code.code.crypto}</span>
                </>
              )}
              {result.code.produced_date && (
                <>
                  <span className="text-muted-foreground">Произведено</span>
                  <span>{result.code.produced_date}</span>
                </>
              )}
              {result.code.expire_date && (
                <>
                  <span className="text-muted-foreground">Годен до</span>
                  <span>{result.code.expire_date}</span>
                </>
              )}
            </div>
          </CardContent>
        </Card>
      )}

      {result.type === "Error" && (
        <Card className="border-red-200 dark:border-red-800 bg-red-50/50 dark:bg-red-950/30">
          <CardContent className="p-5 space-y-2">
            <div className="flex items-center gap-2">
              <XCircle className="w-5 h-5 text-red-500 dark:text-red-400" />
              <span className="font-semibold text-sm text-red-700 dark:text-red-400">
                {result.message}
              </span>
            </div>
            <p className="text-xs text-red-500 dark:text-red-400/80 pl-7">
              {result.explanation}
            </p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
